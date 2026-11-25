use anchor_lang::prelude::*;
use anchor_lang::Key;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::*;
use crate::errors::MarkError;
use crate::events::PauseEvent;
use crate::state::{Exchange, ExchangeParams, Market, RiskUpdate};
use crate::token_util::transfer_as_exchange;

#[derive(Accounts)]
pub struct InitializeExchange<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + Exchange::INIT_SPACE,
        seeds = [b"exchange"],
        bump,
    )]
    pub exchange: Box<Account<'info, Exchange>>,
    #[account(
        init,
        payer = admin,
        mint::decimals = 6,
        mint::authority = exchange,
        mint::freeze_authority = exchange,
    )]
    pub collateral_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = admin,
        token::mint = collateral_mint,
        token::authority = exchange,
        seeds = [b"vault"],
        bump,
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = admin,
        token::mint = collateral_mint,
        token::authority = exchange,
        seeds = [b"insurance"],
        bump,
    )]
    pub insurance_vault: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = admin,
        token::mint = collateral_mint,
        token::authority = exchange,
        seeds = [b"fees"],
        bump,
    )]
    pub fee_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_exchange(
    ctx: Context<InitializeExchange>,
    params: ExchangeParams,
) -> Result<()> {
    let exchange = &mut ctx.accounts.exchange;
    exchange.admin = ctx.accounts.admin.key();
    exchange.collateral_mint = ctx.accounts.collateral_mint.key();
    exchange.vault = ctx.accounts.vault.key();
    exchange.insurance_vault = ctx.accounts.insurance_vault.key();
    exchange.fee_vault = ctx.accounts.fee_vault.key();
    exchange.paused = 0;
    exchange.faucet_enabled = if params.faucet_enabled { 1 } else { 0 };
    exchange.faucet_per_tx = params.faucet_per_tx;
    exchange.faucet_per_user = params.faucet_per_user;
    exchange.next_market_id = 0;
    exchange.insurance_target = params.insurance_target;
    exchange.daily_drain_cap = params.daily_drain_cap;
    exchange.drain_today = 0;
    exchange.drain_day = 0;
    exchange.bump = ctx.bumps.exchange;
    Ok(())
}

#[derive(Accounts)]
pub struct AdminPause<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"exchange"], bump = exchange.bump, has_one = admin)]
    pub exchange: Account<'info, Exchange>,
}

pub fn handle_admin_pause(ctx: Context<AdminPause>, paused: bool) -> Result<()> {
    ctx.accounts.exchange.paused = if paused { 1 } else { 0 };
    emit!(PauseEvent {
        scope: ctx.accounts.exchange.key(),
        paused: ctx.accounts.exchange.paused,
        reason: PAUSE_ADMIN,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct AdminPauseMarket<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [b"exchange"], bump = exchange.bump, has_one = admin)]
    pub exchange: Account<'info, Exchange>,
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
}

pub fn handle_admin_pause_market(ctx: Context<AdminPauseMarket>, paused: bool) -> Result<()> {
    let market = &mut ctx.accounts.market;
    if market.status == STATUS_SETTLED {
        return err!(MarkError::AlreadySettled);
    }
    market.status = if paused { STATUS_PAUSED } else { STATUS_ACTIVE };
    emit!(PauseEvent {
        scope: market.key(),
        paused: if paused { 1 } else { 0 },
        reason: PAUSE_ADMIN,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct AdminSetParams<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"exchange"], bump = exchange.bump, has_one = admin)]
    pub exchange: Account<'info, Exchange>,
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
}

pub fn handle_admin_set_params(ctx: Context<AdminSetParams>, update: RiskUpdate) -> Result<()> {
    crate::math::validate_risk(
        update.im_bps,
        update.im_haircut_bps,
        update.mm_bps,
        update.mm_haircut_bps,
        update.max_leverage,
        update.skew_divisor,
        update.impact_divisor,
        update.funding_divisor,
        update.funding_interval,
        update.taker_fee_bps,
        update.liq_penalty_bps,
        update.keeper_incentive_bps,
        update.max_conf_bps,
        update.max_age_secs,
        TICK + 1,
    )?;
    let market = &mut ctx.accounts.market;
    market.im_bps = update.im_bps;
    market.im_haircut_bps = update.im_haircut_bps;
    market.mm_bps = update.mm_bps;
    market.mm_haircut_bps = update.mm_haircut_bps;
    market.max_leverage = update.max_leverage;
    market.funding_k = update.funding_k;
    market.funding_divisor = update.funding_divisor;
    market.max_funding_premium = update.max_funding_premium;
    market.funding_interval = update.funding_interval;
    market.skew_coeff = update.skew_coeff;
    market.skew_divisor = update.skew_divisor;
    market.impact_coeff = update.impact_coeff;
    market.impact_divisor = update.impact_divisor;
    market.taker_fee_bps = update.taker_fee_bps;
    market.liq_penalty_bps = update.liq_penalty_bps;
    market.keeper_incentive_bps = update.keeper_incentive_bps;
    market.max_age_secs = update.max_age_secs;
    market.max_conf_bps = update.max_conf_bps;
    market.max_oi = update.max_oi;
    market.max_position = update.max_position;
    ctx.accounts.exchange.daily_drain_cap = update.daily_drain_cap;
    ctx.accounts.exchange.faucet_enabled = update.faucet_enabled;
    Ok(())
}

#[derive(Accounts)]
pub struct SeedInsurance<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"exchange"], bump = exchange.bump, has_one = admin)]
    pub exchange: Account<'info, Exchange>,
    #[account(mut, address = exchange.collateral_mint)]
    pub collateral_mint: Account<'info, Mint>,
    #[account(mut, address = exchange.insurance_vault)]
    pub insurance_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn handle_seed_insurance(ctx: Context<SeedInsurance>, amount: u64) -> Result<()> {
    if ctx.accounts.exchange.faucet_enabled == 0 {
        return err!(MarkError::FaucetOff);
    }
    mint_to_exchange(
        &ctx.accounts.token_program,
        &ctx.accounts.collateral_mint,
        &ctx.accounts.insurance_vault,
        &ctx.accounts.exchange,
        amount,
    )
}

pub fn mint_to_exchange<'info>(
    token_program: &Program<'info, Token>,
    mint: &Account<'info, Mint>,
    to: &Account<'info, TokenAccount>,
    exchange: &Account<'info, Exchange>,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let bump = exchange.bump;
    let bump_seed = [bump];
    let seeds: &[&[u8]] = &[b"exchange", &bump_seed];
    anchor_spl::token::mint_to(
        CpiContext::new_with_signer(
            token_program.key(),
            anchor_spl::token::MintTo {
                mint: mint.to_account_info(),
                to: to.to_account_info(),
                authority: exchange.to_account_info(),
            },
            &[seeds],
        ),
        amount,
    )
}

/// Move `amount` of collateral and report whether the daily drain cap broke.
pub fn pay_from_insurance<'info>(
    exchange: &mut Exchange,
    amount: u64,
    now: i64,
    token_program: AccountInfo<'info>,
    insurance: AccountInfo<'info>,
    vault: AccountInfo<'info>,
    authority: AccountInfo<'info>,
) -> Result<bool> {
    transfer_as_exchange(
        token_program,
        insurance,
        vault,
        authority,
        exchange.bump,
        amount,
    )?;
    Ok(note_drain(exchange, amount, now))
}

pub fn note_drain(exchange: &mut Exchange, amount: u64, now: i64) -> bool {
    if amount == 0 {
        return false;
    }
    let day = now / 86_400;
    if exchange.drain_day != day {
        exchange.drain_day = day;
        exchange.drain_today = 0;
    }
    exchange.drain_today = exchange.drain_today.saturating_add(amount);
    exchange.daily_drain_cap > 0 && exchange.drain_today > exchange.daily_drain_cap
}

pub fn pause_for_drain(exchange: &mut Exchange) {
    if exchange.paused == 0 {
        exchange.paused = 1;
        emit!(PauseEvent {
            scope: crate::ID,
            paused: 1,
            reason: PAUSE_DRAIN,
        });
    }
}
