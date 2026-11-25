use anchor_lang::prelude::*;

#[event]
pub struct DepositEvent {
    pub owner: Pubkey,
    pub amount: u64,
    pub free_after: u64,
}

#[event]
pub struct WithdrawEvent {
    pub owner: Pubkey,
    pub amount: u64,
    pub free_after: u64,
}

#[event]
pub struct FillEvent {
    pub market: Pubkey,
    pub owner: Pubkey,
    pub kind: u8,
    pub side: u8,
    pub size_delta: u64,
    pub size_after: i64,
    pub fill_price: u64,
    pub entry_price: u64,
    pub mark: u64,
    pub index_price: u64,
    pub fee: u64,
    pub trade_pnl: i64,
    pub funding_payment: i64,
    pub margin_after: u64,
    pub free_after: u64,
    pub ts: i64,
}

#[event]
pub struct FundingEvent {
    pub market: Pubkey,
    pub mark: u64,
    pub index_price: u64,
    pub premium: i64,
    pub intervals: u16,
    pub cumulative: i128,
    pub ts: i64,
}

#[event]
pub struct LiquidationEvent {
    pub market: Pubkey,
    pub owner: Pubkey,
    pub liquidator: Pubkey,
    pub size: i64,
    pub fill_price: u64,
    pub equity: i64,
    pub penalty: u64,
    pub keeper_fee: u64,
    pub leftover: u64,
    pub shortfall: u64,
    pub ts: i64,
}

#[event]
pub struct OracleEvent {
    pub market: Pubkey,
    pub index_price: u64,
    pub conf: u64,
    pub spot: u64,
    pub source: u8,
    pub ts: i64,
}

#[event]
pub struct PauseEvent {
    pub scope: Pubkey,
    pub paused: u8,
    pub reason: u8,
}

#[event]
pub struct SettleEvent {
    pub market: Pubkey,
    pub price: u64,
    pub ts: i64,
}

#[event]
pub struct ShortfallEvent {
    pub market: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
}
