use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::constants::*;
use crate::errors::MarkError;
use crate::events::{FundingEvent, OracleEvent, SettleEvent, ShortfallEvent};
use crate::math::{self, conf_bps, mark_from_index, model_probability, pyth_to_usd6};
use crate::state::{
    Exchange, FundingPrint, Market, MarketParams, OraclePrint, write_fixed,
};

#[derive(Accounts)]
#[instruction(market_id: u16)]
pub struct InitializeMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"exchange"], bump = exchange.bump, has_one = admin)]
    pub exchange: Box<Account<'info, Exchange>>,
    #[account(
        init,
        payer = admin,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market".as_ref(), market_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub market: Box<Account<'info, Market>>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_market(
    ctx: Context<InitializeMarket>,
    market_id: u16,
    params: MarketParams,
) -> Result<()> {
    math::validate_risk(
        params.im_bps,
        params.im_haircut_bps,
        params.mm_bps,
        params.mm_haircut_bps,
        params.max_leverage,
        params.skew_divisor,
        params.impact_divisor,
        params.funding_divisor,
        params.funding_interval,
        params.taker_fee_bps,
        params.liq_penalty_bps,
        params.keeper_incentive_bps,
        params.max_conf_bps,
        params.max_age_secs,
        params.index_price,
    )?;
    if market_id != ctx.accounts.exchange.next_market_id {
        return err!(MarkError::BadParams);
    }
    if params.oracle_kind > ORACLE_PYTH {
        return err!(MarkError::WrongOracle);
    }
    if params.max_oi == 0 || params.max_position == 0 {
        return err!(MarkError::BadParams);
    }
    let now = Clock::get()?.unix_timestamp;
    let id = ctx.accounts.exchange.next_market_id;
    let market = &mut ctx.accounts.market;
    market.market_id = id;
    write_fixed(&mut market.symbol, &params.symbol)?;
    write_fixed(&mut market.question, &params.question)?;
    write_fixed(&mut market.resolution, &params.resolution)?;
    market.oracle_kind = params.oracle_kind;
    market.pyth_feed_id = params.pyth_feed_id;
    market.strike_price = params.strike_price;
    market.expiry_ts = params.expiry_ts;
    market.status = STATUS_ACTIVE;
    market.index_price = params.index_price;
    market.oracle_conf = params.oracle_conf;
    market.oracle_spot = params.oracle_spot;
    market.oracle_ts = now;
    market.oracle_source = SOURCE_ADMIN;
    market.last_funding_ts = now;
    market.cumulative_funding = 0;
    market.last_premium = 0;
    market.oi_long = 0;
    market.oi_short = 0;
    market.skew_coeff = params.skew_coeff;
    market.skew_divisor = params.skew_divisor;
    market.impact_coeff = params.impact_coeff;
    market.impact_divisor = params.impact_divisor;
    market.im_bps = params.im_bps;
    market.im_haircut_bps = params.im_haircut_bps;
    market.mm_bps = params.mm_bps;
    market.mm_haircut_bps = params.mm_haircut_bps;
    market.max_leverage = params.max_leverage;
    market.funding_k = params.funding_k;
    market.funding_divisor = params.funding_divisor;
    market.max_funding_premium = params.max_funding_premium;
    market.funding_interval = params.funding_interval;
    market.max_oi = params.max_oi;
    market.max_position = params.max_position;
    market.taker_fee_bps = params.taker_fee_bps;
    market.liq_penalty_bps = params.liq_penalty_bps;
    market.keeper_incentive_bps = params.keeper_incentive_bps;
    market.max_age_secs = params.max_age_secs;
    market.max_conf_bps = params.max_conf_bps;
    market.settled_price = 0;
    market.bump = ctx.bumps.market;
    market.mark_price = mark_from_index(
        params.index_price,
        0,
        0,
        params.skew_coeff,
        params.skew_divisor,
    )?;
    let market_key = market.key();
    record_oracle(market, market_key, now, SOURCE_ADMIN)?;
    ctx.accounts.exchange.next_market_id = id.checked_add(1).ok_or(MarkError::Math)?;
    Ok(())
}

#[derive(Accounts)]
pub struct PushAdminIndex<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [b"exchange"], bump = exchange.bump, has_one = admin)]
    pub exchange: Account<'info, Exchange>,
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
}

pub fn handle_push_admin_index(
    ctx: Context<PushAdminIndex>,
    index_price: u64,
    conf: u64,
    spot: u64,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    if market.status == STATUS_SETTLED {
        return err!(MarkError::AlreadySettled);
    }
    if index_price < TICK || index_price > PRICE_SCALE - TICK {
        return err!(MarkError::BadOracle);
    }
    let width = conf_bps(conf.max(1), index_price.max(1))?;
    // A zero confidence is a point publish. Anything wider than the cap is rejected
    // and, because a failed transaction cannot store the row, the client tape keeps it.
    if conf > 0 && width > market.max_conf_bps as u64 {
        return err!(MarkError::WideConfidence);
    }
    let now = Clock::get()?.unix_timestamp;
    market.index_price = index_price;
    market.oracle_conf = conf;
    market.oracle_spot = spot;
    market.oracle_ts = now;
    market.oracle_source = SOURCE_ADMIN;
    market.mark_price = mark_from_index(
        market.index_price,
        market.oi_long,
        market.oi_short,
        market.skew_coeff,
        market.skew_divisor,
    )?;
    let market_key = market.key();
    record_oracle(market, market_key, now, SOURCE_ADMIN)?;
    Ok(())
}

#[derive(Accounts)]
pub struct ConsumePyth<'info> {
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
    pub price_update: Account<'info, PriceUpdateV2>,
}

pub fn handle_consume_pyth(ctx: Context<ConsumePyth>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    if market.oracle_kind != ORACLE_PYTH {
        return err!(MarkError::WrongOracle);
    }
    if market.status == STATUS_SETTLED {
        return err!(MarkError::AlreadySettled);
    }
    if market.strike_price == 0 {
        return err!(MarkError::BadParams);
    }
    let clock = Clock::get()?;
    let price = ctx
        .accounts
        .price_update
        .get_price_no_older_than(&clock, market.max_age_secs as u64, &market.pyth_feed_id)
        .map_err(|_| error!(MarkError::StaleOracle))?;
    let spot = pyth_to_usd6(price.price, price.exponent)?;
    let conf = pyth_to_usd6(price.conf as i64, price.exponent).unwrap_or(u64::MAX);
    let width = conf_bps(conf, spot)?;
    if width > market.max_conf_bps as u64 {
        return err!(MarkError::WideConfidence);
    }
    let index = model_probability(spot, market.strike_price)?;
    let now = clock.unix_timestamp;
    market.index_price = index;
    market.oracle_conf = conf;
    market.oracle_spot = spot;
    market.oracle_ts = now;
    market.oracle_source = SOURCE_PYTH;
    market.mark_price = mark_from_index(
        index,
        market.oi_long,
        market.oi_short,
        market.skew_coeff,
        market.skew_divisor,
    )?;
    let market_key = market.key();
    record_oracle(market, market_key, now, SOURCE_PYTH)?;
    Ok(())
}

#[derive(Accounts)]
pub struct UpdateFunding<'info> {
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
}

pub fn handle_update_funding(ctx: Context<UpdateFunding>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let market_key = ctx.accounts.market.key();
    let market = &mut ctx.accounts.market;
    roll_expiry(market, now);
    if market.status == STATUS_EXPIRED || market.status == STATUS_SETTLED {
        return err!(MarkError::MarketNotActive);
    }
    // Pause stops new funding so a halted market does not keep charging.
    if market.status == STATUS_PAUSED {
        return err!(MarkError::MarketNotActive);
    }
    require_fresh(market, now)?;
    let elapsed = now
        .checked_sub(market.last_funding_ts)
        .ok_or(MarkError::Math)?;
    if elapsed < market.funding_interval {
        return err!(MarkError::FundingEarly);
    }
    let intervals = (elapsed / market.funding_interval).min(MAX_FUNDING_INTERVALS);
    if intervals <= 0 {
        return err!(MarkError::FundingEarly);
    }
    market.mark_price = mark_from_index(
        market.index_price,
        market.oi_long,
        market.oi_short,
        market.skew_coeff,
        market.skew_divisor,
    )?;
    let premium = math::funding_premium(
        market.mark_price,
        market.index_price,
        market.funding_k,
        market.funding_divisor,
        market.max_funding_premium,
    )?;
    let add = (premium as i128)
        .checked_mul(intervals as i128)
        .ok_or(error!(MarkError::Math))?;
    market.cumulative_funding = market
        .cumulative_funding
        .checked_add(add)
        .ok_or(error!(MarkError::Math))?;
    market.last_funding_ts = market
        .last_funding_ts
        .checked_add(
            intervals
                .checked_mul(market.funding_interval)
                .ok_or(error!(MarkError::Math))?,
        )
        .ok_or(error!(MarkError::Math))?;
    market.last_premium = premium;
    let mark_now = market.mark_price;
    let index_now = market.index_price;
    let intervals_u = intervals as u16;
    market.push_funding(FundingPrint {
        ts: now,
        mark: mark_now,
        index_price: index_now,
        premium,
        intervals: intervals_u,
    });
    let cumulative = market.cumulative_funding;
    emit!(FundingEvent {
        market: market_key,
        mark: mark_now,
        index_price: index_now,
        premium,
        intervals: intervals_u,
        cumulative,
        ts: now,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct SettleExpired<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [b"exchange"], bump = exchange.bump, has_one = admin)]
    pub exchange: Account<'info, Exchange>,
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
}

pub fn handle_settle_expired(ctx: Context<SettleExpired>, settlement_price: u64) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let market = &mut ctx.accounts.market;
    if market.status == STATUS_SETTLED {
        return err!(MarkError::AlreadySettled);
    }
    if market.expiry_ts <= 0 || now < market.expiry_ts {
        return err!(MarkError::NotExpired);
    }
    let price = math::clamp_price(settlement_price as i128);
    market.status = STATUS_SETTLED;
    market.settled_price = price;
    market.index_price = price;
    market.mark_price = price;
    market.oracle_ts = now;
    market.oracle_source = SOURCE_ADMIN;
    let market_key = market.key();
    record_oracle(market, market_key, now, SOURCE_ADMIN)?;
    emit!(SettleEvent {
        market: market.key(),
        price,
        ts: now,
    });
    Ok(())
}

pub fn roll_expiry(market: &mut Market, now: i64) {
    if market.status == STATUS_ACTIVE && market.expiry_ts > 0 && now >= market.expiry_ts {
        market.status = STATUS_EXPIRED;
    }
}

pub fn require_fresh(market: &Market, now: i64) -> Result<()> {
    if market.status == STATUS_SETTLED {
        return Ok(());
    }
    if !math::oracle_is_fresh(market.oracle_ts, now, market.max_age_secs) {
        return err!(MarkError::StaleOracle);
    }
    Ok(())
}

pub fn require_open_allowed(market: &Market, exchange_paused: u8) -> Result<()> {
    if exchange_paused != 0 {
        return err!(MarkError::ExchangePaused);
    }
    if market.status != STATUS_ACTIVE {
        return err!(MarkError::MarketNotActive);
    }
    Ok(())
}

fn record_oracle(market: &mut Market, market_key: Pubkey, now: i64, source: u8) -> Result<()> {
    let index_price = market.index_price;
    let conf = market.oracle_conf;
    let spot = market.oracle_spot;
    market.push_oracle(OraclePrint {
        ts: now,
        index_price,
        conf,
        spot,
        source,
    });
    emit!(OracleEvent {
        market: market_key,
        index_price,
        conf,
        spot,
        source,
        ts: now,
    });
    Ok(())
}

pub fn flag_shortfall(market: &mut Market, market_key: Pubkey, owner: Pubkey, amount: u64) {
    if amount == 0 {
        return;
    }
    if market.status == STATUS_ACTIVE {
        market.status = STATUS_PAUSED;
    }
    emit!(ShortfallEvent {
        market: market_key,
        owner,
        amount,
    });
}
