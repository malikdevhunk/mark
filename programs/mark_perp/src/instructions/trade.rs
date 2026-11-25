use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::constants::*;
use crate::errors::MarkError;
use crate::events::{FillEvent, LiquidationEvent};
use crate::math::{self, mark_from_index};
use crate::state::{Exchange, Market, Position, User};
use crate::token_util::transfer_as_exchange;

use super::exchange::{note_drain, pause_for_drain};
use super::market::{flag_shortfall, require_fresh, require_open_allowed, roll_expiry};

#[derive(Accounts)]
pub struct OpenOrIncrease<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"exchange"], bump = exchange.bump)]
    pub exchange: Account<'info, Exchange>,
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
    #[account(
        mut,
        seeds = [b"user", owner.key().as_ref()],
        bump = user.bump,
    )]
    pub user: Account<'info, User>,
    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + Position::INIT_SPACE,
        seeds = [b"position", market.key().as_ref(), owner.key().as_ref()],
        bump,
    )]
    pub position: Account<'info, Position>,
    #[account(mut, address = exchange.vault)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, address = exchange.insurance_vault)]
    pub insurance_vault: Account<'info, TokenAccount>,
    #[account(mut, address = exchange.fee_vault)]
    pub fee_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_open_or_increase(
    ctx: Context<OpenOrIncrease>,
    side: u8,
    size: u64,
    limit_price: u64,
) -> Result<()> {
    if size == 0 || (side != SIDE_LONG && side != SIDE_SHORT) {
        return err!(MarkError::ZeroSize);
    }
    let now = Clock::get()?.unix_timestamp;
    let exchange_paused = ctx.accounts.exchange.paused;
    let bump = ctx.accounts.exchange.bump;
    let owner_key = ctx.accounts.owner.key();

    {
        let market = &mut ctx.accounts.market;
        roll_expiry(market, now);
        require_open_allowed(market, exchange_paused)?;
        require_fresh(market, now)?;
    }

    prepare_position(
        &mut ctx.accounts.position,
        &ctx.accounts.market,
        owner_key,
        ctx.bumps.position,
    )?;

    let mut insurance_balance = ctx.accounts.insurance_vault.amount;
    let market_key = ctx.accounts.market.key();
    let token_program = ctx.accounts.token_program.to_account_info();
    let insurance_ai = ctx.accounts.insurance_vault.to_account_info();
    let vault_ai = ctx.accounts.vault.to_account_info();
    let fee_ai = ctx.accounts.fee_vault.to_account_info();
    let authority = ctx.accounts.exchange.to_account_info();
    let funding_payment = settle_funding(
        &mut ctx.accounts.exchange,
        &mut ctx.accounts.market,
        market_key,
        &mut ctx.accounts.position,
        &mut ctx.accounts.user,
        now,
        &mut insurance_balance,
        token_program.clone(),
        insurance_ai.clone(),
        vault_ai.clone(),
        authority.clone(),
    )?;

    let long = side == SIDE_LONG;
    let position = &ctx.accounts.position;
    if position.size != 0 && ((position.size > 0) != long) {
        return err!(MarkError::WrongSide);
    }

    let market = &ctx.accounts.market;
    let mark = mark_from_index(
        market.index_price,
        market.oi_long,
        market.oi_short,
        market.skew_coeff,
        market.skew_divisor,
    )?;
    let fill = math::fill_from_mark(
        mark,
        long,
        size,
        market.impact_coeff,
        market.impact_divisor,
    )?;
    if long {
        if fill > limit_price {
            return err!(MarkError::Slippage);
        }
    } else if fill < limit_price {
        return err!(MarkError::Slippage);
    }

    let old_abs = position.size.unsigned_abs();
    let new_abs = old_abs.checked_add(size).ok_or(error!(MarkError::Math))?;
    let new_entry = if old_abs == 0 {
        fill
    } else {
        math::weighted_entry(old_abs, position.entry_price, size, fill)?
    };
    let new_im = math::margin_requirement(
        new_abs,
        new_entry,
        long,
        market.im_bps,
        market.im_haircut_bps,
        market.max_leverage,
    )?;
    let fee = math::taker_fee(size, market.taker_fee_bps)?;
    let extra = new_im.saturating_sub(position.margin);
    if ctx.accounts.user.free_collateral < extra.checked_add(fee).ok_or(error!(MarkError::Math))? {
        return err!(MarkError::InsufficientFree);
    }
    let new_oi = if long {
        market.oi_long.checked_add(size)
    } else {
        market.oi_short.checked_add(size)
    }
    .ok_or(error!(MarkError::Math))?;
    if new_oi > market.max_oi {
        return err!(MarkError::OpenInterestCap);
    }
    if new_abs > market.max_position {
        return err!(MarkError::PositionCap);
    }

    ctx.accounts.user.free_collateral -= extra + fee;
    transfer_as_exchange(token_program, vault_ai, fee_ai, authority, bump, fee)?;

    let position = &mut ctx.accounts.position;
    if position.open_ts == 0 {
        position.open_ts = now;
    }
    position.margin = position.margin.checked_add(extra).ok_or(error!(MarkError::Math))?;
    position.entry_price = new_entry;
    position.size = if long {
        position.size.checked_add(size as i64)
    } else {
        position.size.checked_sub(size as i64)
    }
    .ok_or(error!(MarkError::Math))?;
    position.last_touch_ts = now;

    let market = &mut ctx.accounts.market;
    if long {
        market.oi_long = new_oi;
    } else {
        market.oi_short = new_oi;
    }
    market.mark_price = mark_from_index(
        market.index_price,
        market.oi_long,
        market.oi_short,
        market.skew_coeff,
        market.skew_divisor,
    )?;

    emit!(FillEvent {
        market: market.key(),
        owner: owner_key,
        kind: FILL_OPEN,
        side,
        size_delta: size,
        size_after: position.size,
        fill_price: fill,
        entry_price: position.entry_price,
        mark: market.mark_price,
        index_price: market.index_price,
        fee,
        trade_pnl: 0,
        funding_payment,
        margin_after: position.margin,
        free_after: ctx.accounts.user.free_collateral,
        ts: now,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct ReduceOrClose<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"exchange"], bump = exchange.bump)]
    pub exchange: Account<'info, Exchange>,
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
    #[account(
        mut,
        seeds = [b"user", owner.key().as_ref()],
        bump = user.bump,
    )]
    pub user: Account<'info, User>,
    #[account(
        mut,
        seeds = [b"position", market.key().as_ref(), owner.key().as_ref()],
        bump = position.bump,
    )]
    pub position: Account<'info, Position>,
    #[account(mut, address = exchange.vault)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, address = exchange.insurance_vault)]
    pub insurance_vault: Account<'info, TokenAccount>,
    #[account(mut, address = exchange.fee_vault)]
    pub fee_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn handle_reduce_or_close(
    ctx: Context<ReduceOrClose>,
    size: u64,
    limit_price: u64,
) -> Result<()> {
    if size == 0 {
        return err!(MarkError::ZeroSize);
    }
    let now = Clock::get()?.unix_timestamp;
    let bump = ctx.accounts.exchange.bump;
    {
        let market = &mut ctx.accounts.market;
        roll_expiry(market, now);
        if market.status == STATUS_SETTLED {
            // Closes at the settlement price do not need a live oracle.
        } else {
            require_fresh(market, now)?;
        }
    }
    if ctx.accounts.position.size == 0 {
        return err!(MarkError::FlatPosition);
    }
    let old_abs = ctx.accounts.position.size.unsigned_abs();
    if size > old_abs {
        return err!(MarkError::CloseTooLarge);
    }
    let closing_long = ctx.accounts.position.size > 0;

    let mut insurance_balance = ctx.accounts.insurance_vault.amount;
    let market_key = ctx.accounts.market.key();
    let token_program = ctx.accounts.token_program.to_account_info();
    let insurance_ai = ctx.accounts.insurance_vault.to_account_info();
    let vault_ai = ctx.accounts.vault.to_account_info();
    let fee_ai = ctx.accounts.fee_vault.to_account_info();
    let authority = ctx.accounts.exchange.to_account_info();
    let funding_payment = settle_funding(
        &mut ctx.accounts.exchange,
        &mut ctx.accounts.market,
        market_key,
        &mut ctx.accounts.position,
        &mut ctx.accounts.user,
        now,
        &mut insurance_balance,
        token_program.clone(),
        insurance_ai.clone(),
        vault_ai.clone(),
        authority.clone(),
    )?;

    let market = &ctx.accounts.market;
    let fill = if market.status == STATUS_SETTLED {
        market.settled_price
    } else {
        let mark = mark_from_index(
            market.index_price,
            market.oi_long,
            market.oi_short,
            market.skew_coeff,
            market.skew_divisor,
        )?;
        // Closing a long sells, closing a short buys.
        math::fill_from_mark(
            mark,
            !closing_long,
            size,
            market.impact_coeff,
            market.impact_divisor,
        )?
    };
    if closing_long {
        if fill < limit_price {
            return err!(MarkError::Slippage);
        }
    } else if fill > limit_price {
        return err!(MarkError::Slippage);
    }

    let fee = math::taker_fee(size, market.taker_fee_bps)?;
    let entry = ctx.accounts.position.entry_price;
    let closed_signed: i64 = if closing_long { size as i64 } else { -(size as i64) };
    let trade_pnl = math::pnl(closed_signed, entry, fill)?;
    let release = if size == ctx.accounts.position.size.unsigned_abs() {
        ctx.accounts.position.margin
    } else {
        math::mul_div(ctx.accounts.position.margin, size, old_abs)?
    };

    ctx.accounts.position.margin -= release;
    let (mut returned, shortfall) = realize_pnl(
        &mut ctx.accounts.exchange,
        trade_pnl,
        release,
        now,
        &mut insurance_balance,
        token_program.clone(),
        insurance_ai.clone(),
        vault_ai.clone(),
        authority.clone(),
    )?;
    if shortfall > 0 {
        let market_key = ctx.accounts.market.key();
        let owner = ctx.accounts.owner.key();
        flag_shortfall(&mut ctx.accounts.market, market_key, owner, shortfall);
    }

    charge_fee(
        &mut ctx.accounts.user,
        &mut returned,
        fee,
        bump,
        token_program,
        vault_ai,
        fee_ai,
        authority,
    )?;
    ctx.accounts.user.free_collateral = ctx
        .accounts
        .user
        .free_collateral
        .checked_add(returned)
        .ok_or(error!(MarkError::Math))?;
    add_realised(&mut ctx.accounts.user, trade_pnl, funding_payment, fee as i64)?;

    apply_size_reduction(&mut ctx.accounts.position, &mut ctx.accounts.market, size, closing_long, now)?;

    let position = &ctx.accounts.position;
    let market = &ctx.accounts.market;
    emit!(FillEvent {
        market: market.key(),
        owner: ctx.accounts.owner.key(),
        kind: FILL_CLOSE,
        side: if closing_long { SIDE_LONG } else { SIDE_SHORT },
        size_delta: size,
        size_after: position.size,
        fill_price: fill,
        entry_price: entry,
        mark: market.mark_price,
        index_price: market.index_price,
        fee,
        trade_pnl,
        funding_payment,
        margin_after: position.margin,
        free_after: ctx.accounts.user.free_collateral,
        ts: now,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,
    /// CHECK: position owner. The position PDA binds this key.
    pub owner: UncheckedAccount<'info>,
    #[account(mut, seeds = [b"exchange"], bump = exchange.bump)]
    pub exchange: Account<'info, Exchange>,
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
    #[account(
        mut,
        seeds = [b"user", owner.key().as_ref()],
        bump = user.bump,
    )]
    pub user: Account<'info, User>,
    #[account(
        mut,
        seeds = [b"position", market.key().as_ref(), owner.key().as_ref()],
        bump = position.bump,
    )]
    pub position: Account<'info, Position>,
    #[account(mut, address = exchange.vault)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, address = exchange.insurance_vault)]
    pub insurance_vault: Account<'info, TokenAccount>,
    #[account(address = exchange.collateral_mint)]
    pub collateral_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = liquidator,
        associated_token::mint = collateral_mint,
        associated_token::authority = liquidator,
    )]
    pub liquidator_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_liquidate(ctx: Context<Liquidate>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let bump = ctx.accounts.exchange.bump;
    {
        let market = &mut ctx.accounts.market;
        roll_expiry(market, now);
        if market.status == STATUS_SETTLED {
            return err!(MarkError::AlreadySettled);
        }
        require_fresh(market, now)?;
    }
    if ctx.accounts.position.size == 0 {
        return err!(MarkError::FlatPosition);
    }

    let mut insurance_balance = ctx.accounts.insurance_vault.amount;
    let market_key = ctx.accounts.market.key();
    let token_program = ctx.accounts.token_program.to_account_info();
    let insurance_ai = ctx.accounts.insurance_vault.to_account_info();
    let vault_ai = ctx.accounts.vault.to_account_info();
    let authority = ctx.accounts.exchange.to_account_info();
    let keeper_ai = ctx.accounts.liquidator_ata.to_account_info();
    let funding_payment = settle_funding(
        &mut ctx.accounts.exchange,
        &mut ctx.accounts.market,
        market_key,
        &mut ctx.accounts.position,
        &mut ctx.accounts.user,
        now,
        &mut insurance_balance,
        token_program.clone(),
        insurance_ai.clone(),
        vault_ai.clone(),
        authority.clone(),
    )?;

    let closing_long = ctx.accounts.position.size > 0;
    let size = ctx.accounts.position.size.unsigned_abs();
    let entry = ctx.accounts.position.entry_price;
    let margin = ctx.accounts.position.margin;
    let market = &mut ctx.accounts.market;
    market.mark_price = mark_from_index(
        market.index_price,
        market.oi_long,
        market.oi_short,
        market.skew_coeff,
        market.skew_divisor,
    )?;
    let fill = market.mark_price;
    let trade_pnl = math::pnl(ctx.accounts.position.size, entry, fill)?;
    let equity = (margin as i128) + (trade_pnl as i128);
    let mm = math::margin_requirement(
        size,
        fill,
        closing_long,
        market.mm_bps,
        market.mm_haircut_bps,
        0,
    )?;
    if equity >= mm as i128 {
        return err!(MarkError::NotLiquidatable);
    }

    ctx.accounts.position.margin = 0;
    let (returned, pnl_shortfall) = realize_pnl(
        &mut ctx.accounts.exchange,
        trade_pnl,
        margin,
        now,
        &mut insurance_balance,
        token_program.clone(),
        insurance_ai.clone(),
        vault_ai.clone(),
        authority.clone(),
    )?;
    if pnl_shortfall > 0 {
        let owner = ctx.accounts.owner.key();
        let market_key = ctx.accounts.market.key();
        flag_shortfall(&mut ctx.accounts.market, market_key, owner, pnl_shortfall);
    }

    let penalty_full = math::liquidation_penalty(size, ctx.accounts.market.liq_penalty_bps)?;
    let penalty = penalty_full.min(returned);
    let keeper_fee = math::mul_div(
        penalty,
        ctx.accounts.market.keeper_incentive_bps as u64,
        BPS,
    )?;
    let insurance_part = penalty - keeper_fee;
    let leftover = returned - penalty;

    transfer_as_exchange(
        token_program.clone(),
        vault_ai.clone(),
        keeper_ai,
        authority.clone(),
        bump,
        keeper_fee,
    )?;
    transfer_as_exchange(
        token_program,
        vault_ai,
        insurance_ai,
        authority,
        bump,
        insurance_part,
    )?;
    ctx.accounts.user.free_collateral = ctx
        .accounts
        .user
        .free_collateral
        .checked_add(leftover)
        .ok_or(error!(MarkError::Math))?;
    add_realised(
        &mut ctx.accounts.user,
        trade_pnl,
        funding_payment,
        penalty as i64,
    )?;

    apply_size_reduction(
        &mut ctx.accounts.position,
        &mut ctx.accounts.market,
        size,
        closing_long,
        now,
    )?;

    let equity_i = i64::try_from(equity).unwrap_or(i64::MIN);
    emit!(LiquidationEvent {
        market: ctx.accounts.market.key(),
        owner: ctx.accounts.owner.key(),
        liquidator: ctx.accounts.liquidator.key(),
        size: if closing_long { size as i64 } else { -(size as i64) },
        fill_price: fill,
        equity: equity_i,
        penalty,
        keeper_fee,
        leftover,
        shortfall: pnl_shortfall,
        ts: now,
    });
    emit!(FillEvent {
        market: ctx.accounts.market.key(),
        owner: ctx.accounts.owner.key(),
        kind: FILL_LIQUIDATION,
        side: if closing_long { SIDE_LONG } else { SIDE_SHORT },
        size_delta: size,
        size_after: 0,
        fill_price: fill,
        entry_price: entry,
        mark: ctx.accounts.market.mark_price,
        index_price: ctx.accounts.market.index_price,
        fee: penalty,
        trade_pnl,
        funding_payment,
        margin_after: 0,
        free_after: ctx.accounts.user.free_collateral,
        ts: now,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct ApplyFunding<'info> {
    #[account(mut, seeds = [b"exchange"], bump = exchange.bump)]
    pub exchange: Account<'info, Exchange>,
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
    /// CHECK: position owner, bound by the position PDA.
    pub owner: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"user", owner.key().as_ref()],
        bump = user.bump,
    )]
    pub user: Account<'info, User>,
    #[account(
        mut,
        seeds = [b"position", market.key().as_ref(), owner.key().as_ref()],
        bump = position.bump,
    )]
    pub position: Account<'info, Position>,
    #[account(mut, address = exchange.vault)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, address = exchange.insurance_vault)]
    pub insurance_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn handle_apply_funding(ctx: Context<ApplyFunding>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    if ctx.accounts.position.size == 0 {
        ctx.accounts.position.last_cumulative_funding = ctx.accounts.market.cumulative_funding;
        return Ok(());
    }
    let mut insurance_balance = ctx.accounts.insurance_vault.amount;
    let market_key = ctx.accounts.market.key();
    let token_program = ctx.accounts.token_program.to_account_info();
    let insurance_ai = ctx.accounts.insurance_vault.to_account_info();
    let vault_ai = ctx.accounts.vault.to_account_info();
    let authority = ctx.accounts.exchange.to_account_info();
    settle_funding(
        &mut ctx.accounts.exchange,
        &mut ctx.accounts.market,
        market_key,
        &mut ctx.accounts.position,
        &mut ctx.accounts.user,
        now,
        &mut insurance_balance,
        token_program,
        insurance_ai,
        vault_ai,
        authority,
    )?;
    Ok(())
}

fn prepare_position(
    position: &mut Position,
    market: &Account<Market>,
    owner: Pubkey,
    bump: u8,
) -> Result<()> {
    if position.owner == Pubkey::default() {
        position.owner = owner;
        position.market = market.key();
        position.size = 0;
        position.entry_price = 0;
        position.margin = 0;
        position.last_cumulative_funding = market.cumulative_funding;
        position.open_ts = 0;
        position.last_touch_ts = 0;
        position.bump = bump;
    } else if position.owner != owner || position.market != market.key() {
        return err!(MarkError::Unauthorized);
    }
    Ok(())
}

fn settle_funding<'info>(
    exchange: &mut Exchange,
    market: &mut Market,
    market_key: Pubkey,
    position: &mut Position,
    user: &mut User,
    now: i64,
    insurance_balance: &mut u64,
    token_program: AccountInfo<'info>,
    insurance: AccountInfo<'info>,
    vault: AccountInfo<'info>,
    authority: AccountInfo<'info>,
) -> Result<i64> {
    let payment = math::funding_payment(
        position.size,
        market.cumulative_funding,
        position.last_cumulative_funding,
    )?;
    position.last_cumulative_funding = market.cumulative_funding;
    position.last_touch_ts = now;
    if payment == 0 || position.size == 0 {
        return Ok(0);
    }
    if payment > 0 {
        let mut left = payment as u64;
        let from_free = left.min(user.free_collateral);
        user.free_collateral -= from_free;
        left -= from_free;
        let from_margin = left.min(position.margin);
        position.margin -= from_margin;
        left -= from_margin;
        let paid = payment as u64 - left;
        transfer_as_exchange(
            token_program,
            vault,
            insurance,
            authority,
            exchange.bump,
            paid,
        )?;
        if left > 0 {
            flag_shortfall(market, market_key, user.owner, left);
        }
        Ok(payment)
    } else {
        let credit = (-payment) as u64;
        let paid = credit.min(*insurance_balance);
        transfer_as_exchange(
            token_program,
            insurance,
            vault,
            authority,
            exchange.bump,
            paid,
        )?;
        *insurance_balance -= paid;
        user.free_collateral = user
            .free_collateral
            .checked_add(paid)
            .ok_or(error!(MarkError::Math))?;
        if note_drain(exchange, paid, now) {
            pause_for_drain(exchange);
        }
        let shortfall = credit - paid;
        if shortfall > 0 {
            flag_shortfall(market, market_key, user.owner, shortfall);
        }
        Ok(payment)
    }
}

fn realize_pnl<'info>(
    exchange: &mut Exchange,
    trade_pnl: i64,
    release: u64,
    now: i64,
    insurance_balance: &mut u64,
    token_program: AccountInfo<'info>,
    insurance: AccountInfo<'info>,
    vault: AccountInfo<'info>,
    authority: AccountInfo<'info>,
) -> Result<(u64, u64)> {
    if trade_pnl >= 0 {
        let want = trade_pnl as u64;
        let paid = want.min(*insurance_balance);
        transfer_as_exchange(
            token_program,
            insurance,
            vault,
            authority,
            exchange.bump,
            paid,
        )?;
        *insurance_balance -= paid;
        if note_drain(exchange, paid, now) {
            pause_for_drain(exchange);
        }
        Ok((
            release.checked_add(paid).ok_or(error!(MarkError::Math))?,
            want - paid,
        ))
    } else {
        let loss = (-trade_pnl) as u64;
        let take = loss.min(release);
        transfer_as_exchange(
            token_program.clone(),
            vault,
            insurance,
            authority,
            exchange.bump,
            take,
        )?;
        Ok((release - take, loss - take))
    }
}

fn charge_fee<'info>(
    user: &mut User,
    returned: &mut u64,
    fee: u64,
    bump: u8,
    token_program: AccountInfo<'info>,
    vault: AccountInfo<'info>,
    fee_vault: AccountInfo<'info>,
    authority: AccountInfo<'info>,
) -> Result<()> {
    if fee == 0 {
        return Ok(());
    }
    let from_free = fee.min(user.free_collateral);
    user.free_collateral -= from_free;
    let mut left = fee - from_free;
    let from_returned = left.min(*returned);
    *returned -= from_returned;
    left -= from_returned;
    if left > 0 {
        return err!(MarkError::InsufficientFree);
    }
    transfer_as_exchange(token_program, vault, fee_vault, authority, bump, fee)
}

fn add_realised(user: &mut User, trade_pnl: i64, funding_payment: i64, cost: i64) -> Result<()> {
    let delta = (trade_pnl as i128) - (funding_payment as i128) - (cost as i128);
    let delta_i = i64::try_from(delta).unwrap_or(0);
    user.realised_pnl = user.realised_pnl.saturating_add(delta_i);
    Ok(())
}

fn apply_size_reduction(
    position: &mut Position,
    market: &mut Market,
    size: u64,
    closing_long: bool,
    now: i64,
) -> Result<()> {
    if closing_long {
        market.oi_long = market.oi_long.checked_sub(size).ok_or(error!(MarkError::Math))?;
        position.size = position
            .size
            .checked_sub(size as i64)
            .ok_or(error!(MarkError::Math))?;
    } else {
        market.oi_short = market
            .oi_short
            .checked_sub(size)
            .ok_or(error!(MarkError::Math))?;
        position.size = position
            .size
            .checked_add(size as i64)
            .ok_or(error!(MarkError::Math))?;
    }
    if position.size == 0 {
        position.margin = 0;
        position.entry_price = 0;
        position.open_ts = 0;
    }
    position.last_touch_ts = now;
    if market.status == STATUS_SETTLED {
        market.mark_price = market.settled_price;
    } else {
        market.mark_price = mark_from_index(
            market.index_price,
            market.oi_long,
            market.oi_short,
            market.skew_coeff,
            market.skew_divisor,
        )?;
    }
    Ok(())
}
