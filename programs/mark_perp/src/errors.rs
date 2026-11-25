use anchor_lang::prelude::*;

#[error_code]
pub enum MarkError {
    #[msg("Math overflow")]
    Math,
    #[msg("Text does not fit the account field")]
    TextTooLong,
    #[msg("The exchange is paused")]
    ExchangePaused,
    #[msg("This market is not taking new trades")]
    MarketNotActive,
    #[msg("Oracle is stale")]
    StaleOracle,
    #[msg("Oracle confidence is too wide")]
    WideConfidence,
    #[msg("Oracle price is unusable")]
    BadOracle,
    #[msg("Free collateral is too low")]
    InsufficientFree,
    #[msg("Withdraw would spend collateral that is locked as margin")]
    WithdrawExceedsFree,
    #[msg("Size must be greater than zero")]
    ZeroSize,
    #[msg("Close size is larger than the position")]
    CloseTooLarge,
    #[msg("Close this side before opening the other")]
    WrongSide,
    #[msg("Fill is past the limit price")]
    Slippage,
    #[msg("Open interest cap reached")]
    OpenInterestCap,
    #[msg("Position size cap reached")]
    PositionCap,
    #[msg("Funding interval has not elapsed")]
    FundingEarly,
    #[msg("Position is above maintenance margin")]
    NotLiquidatable,
    #[msg("Market has not reached expiry")]
    NotExpired,
    #[msg("Market is already settled")]
    AlreadySettled,
    #[msg("Only the admin can do this")]
    Unauthorized,
    #[msg("Faucet is disabled")]
    FaucetOff,
    #[msg("Faucet cap exceeded")]
    FaucetCap,
    #[msg("Risk parameters are inconsistent")]
    BadParams,
    #[msg("This oracle source does not match the market")]
    WrongOracle,
    #[msg("Nothing to settle on a flat position")]
    FlatPosition,
}
