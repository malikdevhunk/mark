use anchor_lang::prelude::*;

use crate::constants::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, Default)]
pub struct OraclePrint {
    pub ts: i64,
    pub index_price: u64,
    pub conf: u64,
    pub spot: u64,
    pub source: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, Default)]
pub struct FundingPrint {
    pub ts: i64,
    pub mark: u64,
    pub index_price: u64,
    pub premium: i64,
    pub intervals: u16,
}

#[account]
#[derive(InitSpace)]
pub struct Exchange {
    pub admin: Pubkey,
    pub collateral_mint: Pubkey,
    pub vault: Pubkey,
    pub insurance_vault: Pubkey,
    pub fee_vault: Pubkey,
    pub paused: u8,
    pub faucet_enabled: u8,
    pub faucet_per_tx: u64,
    pub faucet_per_user: u64,
    pub next_market_id: u16,
    pub insurance_target: u64,
    pub daily_drain_cap: u64,
    pub drain_today: u64,
    pub drain_day: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Market {
    pub market_id: u16,
    pub symbol: [u8; 16],
    pub question: [u8; 192],
    pub resolution: [u8; 320],
    pub oracle_kind: u8,
    pub pyth_feed_id: [u8; 32],
    pub strike_price: u64,
    pub expiry_ts: i64,
    pub status: u8,
    pub index_price: u64,
    pub mark_price: u64,
    pub oracle_conf: u64,
    pub oracle_spot: u64,
    pub oracle_ts: i64,
    pub oracle_source: u8,
    pub last_funding_ts: i64,
    pub cumulative_funding: i128,
    pub last_premium: i64,
    pub oi_long: u64,
    pub oi_short: u64,
    pub skew_coeff: u64,
    pub skew_divisor: u64,
    pub impact_coeff: u64,
    pub impact_divisor: u64,
    pub im_bps: u16,
    pub im_haircut_bps: u16,
    pub mm_bps: u16,
    pub mm_haircut_bps: u16,
    pub max_leverage: u16,
    pub funding_k: u64,
    pub funding_divisor: u64,
    pub max_funding_premium: u64,
    pub funding_interval: i64,
    pub max_oi: u64,
    pub max_position: u64,
    pub taker_fee_bps: u16,
    pub liq_penalty_bps: u16,
    pub keeper_incentive_bps: u16,
    pub max_age_secs: u32,
    pub max_conf_bps: u16,
    pub settled_price: u64,
    pub bump: u8,
    pub oracle_len: u8,
    pub oracle_head: u8,
    pub funding_len: u8,
    pub funding_head: u8,
    pub prints: [OraclePrint; ORACLE_PRINTS],
    pub funding_prints: [FundingPrint; FUNDING_PRINTS],
}

#[account]
#[derive(InitSpace)]
pub struct User {
    pub owner: Pubkey,
    pub free_collateral: u64,
    pub realised_pnl: i64,
    pub faucet_claimed: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Position {
    pub owner: Pubkey,
    pub market: Pubkey,
    pub size: i64,
    pub entry_price: u64,
    pub margin: u64,
    pub last_cumulative_funding: i128,
    pub open_ts: i64,
    pub last_touch_ts: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ExchangeParams {
    pub faucet_enabled: bool,
    pub faucet_per_tx: u64,
    pub faucet_per_user: u64,
    pub insurance_target: u64,
    pub daily_drain_cap: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MarketParams {
    pub symbol: String,
    pub question: String,
    pub resolution: String,
    pub oracle_kind: u8,
    pub pyth_feed_id: [u8; 32],
    pub strike_price: u64,
    pub expiry_ts: i64,
    pub index_price: u64,
    pub oracle_conf: u64,
    pub oracle_spot: u64,
    pub skew_coeff: u64,
    pub skew_divisor: u64,
    pub impact_coeff: u64,
    pub impact_divisor: u64,
    pub im_bps: u16,
    pub im_haircut_bps: u16,
    pub mm_bps: u16,
    pub mm_haircut_bps: u16,
    pub max_leverage: u16,
    pub funding_k: u64,
    pub funding_divisor: u64,
    pub max_funding_premium: u64,
    pub funding_interval: i64,
    pub max_oi: u64,
    pub max_position: u64,
    pub taker_fee_bps: u16,
    pub liq_penalty_bps: u16,
    pub keeper_incentive_bps: u16,
    pub max_age_secs: u32,
    pub max_conf_bps: u16,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RiskUpdate {
    pub im_bps: u16,
    pub im_haircut_bps: u16,
    pub mm_bps: u16,
    pub mm_haircut_bps: u16,
    pub max_leverage: u16,
    pub funding_k: u64,
    pub funding_divisor: u64,
    pub max_funding_premium: u64,
    pub funding_interval: i64,
    pub skew_coeff: u64,
    pub skew_divisor: u64,
    pub impact_coeff: u64,
    pub impact_divisor: u64,
    pub taker_fee_bps: u16,
    pub liq_penalty_bps: u16,
    pub keeper_incentive_bps: u16,
    pub max_age_secs: u32,
    pub max_conf_bps: u16,
    pub max_oi: u64,
    pub max_position: u64,
    pub daily_drain_cap: u64,
    pub faucet_enabled: u8,
}

impl Market {
    pub fn push_oracle(&mut self, print: OraclePrint) {
        let n = ORACLE_PRINTS;
        let head = (self.oracle_head as usize) % n;
        self.prints[head] = print;
        self.oracle_head = ((head + 1) % n) as u8;
        if (self.oracle_len as usize) < n {
            self.oracle_len += 1;
        }
    }

    pub fn push_funding(&mut self, print: FundingPrint) {
        let n = FUNDING_PRINTS;
        let head = (self.funding_head as usize) % n;
        self.funding_prints[head] = print;
        self.funding_head = ((head + 1) % n) as u8;
        if (self.funding_len as usize) < n {
            self.funding_len += 1;
        }
    }
}

pub fn write_fixed(dst: &mut [u8], src: &str) -> Result<()> {
    if src.len() > dst.len() {
        return err!(crate::errors::MarkError::TextTooLong);
    }
    dst.fill(0);
    dst[..src.len()].copy_from_slice(src.as_bytes());
    Ok(())
}

