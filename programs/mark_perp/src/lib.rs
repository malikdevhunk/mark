// Copyright (c) 2026 Saddam Malik. MIT License. See LICENSE.
//! Mark — isolated-margin perpetual on an event probability.

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod math;
pub mod state;
pub mod token_util;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::{ExchangeParams, MarketParams, RiskUpdate};

declare_id!("EwCuCZyudnEjbHZ6wFYZtZBnpJaGEYsX2X8VJiMh5MrG");

#[program]
pub mod mark_perp {
    use super::*;

    pub fn initialize_exchange(
        ctx: Context<InitializeExchange>,
        params: ExchangeParams,
    ) -> Result<()> {
        instructions::exchange::handle_initialize_exchange(ctx, params)
    }

    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        market_id: u16,
        params: MarketParams,
    ) -> Result<()> {
        instructions::market::handle_initialize_market(ctx, market_id, params)
    }

    pub fn initialize_user(ctx: Context<InitializeUser>) -> Result<()> {
        instructions::user::handle_initialize_user(ctx)
    }

    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        instructions::user::handle_deposit(ctx, amount)
    }

    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
        instructions::user::handle_withdraw(ctx, amount)
    }

    pub fn faucet(ctx: Context<Faucet>, amount: u64) -> Result<()> {
        instructions::user::handle_faucet(ctx, amount)
    }

    pub fn push_admin_index(
        ctx: Context<PushAdminIndex>,
        index_price: u64,
        conf: u64,
        spot: u64,
    ) -> Result<()> {
        instructions::market::handle_push_admin_index(ctx, index_price, conf, spot)
    }

    pub fn consume_pyth(ctx: Context<ConsumePyth>) -> Result<()> {
        instructions::market::handle_consume_pyth(ctx)
    }

    pub fn open_or_increase(
        ctx: Context<OpenOrIncrease>,
        side: u8,
        size: u64,
        limit_price: u64,
    ) -> Result<()> {
        instructions::trade::handle_open_or_increase(ctx, side, size, limit_price)
    }

    pub fn reduce_or_close(
        ctx: Context<ReduceOrClose>,
        size: u64,
        limit_price: u64,
    ) -> Result<()> {
        instructions::trade::handle_reduce_or_close(ctx, size, limit_price)
    }

    pub fn update_funding(ctx: Context<UpdateFunding>) -> Result<()> {
        instructions::market::handle_update_funding(ctx)
    }

    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
        instructions::trade::handle_liquidate(ctx)
    }

    pub fn apply_funding(ctx: Context<ApplyFunding>) -> Result<()> {
        instructions::trade::handle_apply_funding(ctx)
    }

    pub fn admin_pause(ctx: Context<AdminPause>, paused: bool) -> Result<()> {
        instructions::exchange::handle_admin_pause(ctx, paused)
    }

    pub fn admin_pause_market(ctx: Context<AdminPauseMarket>, paused: bool) -> Result<()> {
        instructions::exchange::handle_admin_pause_market(ctx, paused)
    }

    pub fn admin_set_params(ctx: Context<AdminSetParams>, update: RiskUpdate) -> Result<()> {
        instructions::exchange::handle_admin_set_params(ctx, update)
    }

    pub fn settle_expired(ctx: Context<SettleExpired>, settlement_price: u64) -> Result<()> {
        instructions::market::handle_settle_expired(ctx, settlement_price)
    }

    pub fn seed_insurance(ctx: Context<SeedInsurance>, amount: u64) -> Result<()> {
        instructions::exchange::handle_seed_insurance(ctx, amount)
    }
}
