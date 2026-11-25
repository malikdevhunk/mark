/// Prices are micro-USD per 1.00 of probability. 1_000_000 = certain.
pub const PRICE_SCALE: u64 = 1_000_000;
/// Contract size scale. 1_000_000 = one contract. One contract pays $1
/// when probability moves by 1.00, so size units and USDC micro-units match.
pub const SIZE_SCALE: u64 = 1_000_000;
/// Prices are clamped to [TICK, PRICE_SCALE - TICK]. Never 0 or 1.
pub const TICK: u64 = 1_000;
pub const BPS: u64 = 10_000;

pub const STATUS_ACTIVE: u8 = 0;
pub const STATUS_PAUSED: u8 = 1;
pub const STATUS_EXPIRED: u8 = 2;
pub const STATUS_SETTLED: u8 = 3;

pub const ORACLE_ADMIN: u8 = 0;
pub const ORACLE_PYTH: u8 = 1;

pub const SOURCE_ADMIN: u8 = 1;
pub const SOURCE_PYTH: u8 = 2;

pub const SIDE_LONG: u8 = 0;
pub const SIDE_SHORT: u8 = 1;

pub const FILL_OPEN: u8 = 0;
pub const FILL_CLOSE: u8 = 1;
pub const FILL_LIQUIDATION: u8 = 2;

pub const PAUSE_ADMIN: u8 = 0;
pub const PAUSE_SHORTFALL: u8 = 1;
pub const PAUSE_DRAIN: u8 = 2;

/// Impact is half the skew slope, so a round trip with no index move
/// crosses the same price. The taker fee is the spread.
/// One contract of imbalance moves the mark by 0.1 cent.
/// Ten contracts move it by one cent. Impact is half of that slope,
/// so a round trip with no index move crosses the same price.
pub const SKEW_COEFF: u64 = 1_000;
pub const SKEW_DIVISOR: u64 = 1_000_000;
pub const IMPACT_COEFF: u64 = 500;
pub const IMPACT_DIVISOR: u64 = 1_000_000;

pub const IM_BPS: u16 = 500;
pub const IM_HAIRCUT_BPS: u16 = 5_000;
pub const MM_BPS: u16 = 250;
pub const MM_HAIRCUT_BPS: u16 = 2_500;
pub const MAX_LEVERAGE: u16 = 20;

pub const FUNDING_K: u64 = 1;
pub const FUNDING_DIVISOR: u64 = 10;
pub const MAX_FUNDING_PREMIUM: u64 = 50_000;
pub const FUNDING_INTERVAL: i64 = 3_600;

pub const TAKER_FEE_BPS: u16 = 20;
pub const LIQ_PENALTY_BPS: u16 = 100;
pub const KEEPER_INCENTIVE_BPS: u16 = 2_000;

pub const MAX_CONF_BPS: u16 = 250;
pub const ORACLE_PRINTS: usize = 6;
pub const FUNDING_PRINTS: usize = 4;

/// Cap a late crank so one call cannot apply months of funding.
pub const MAX_FUNDING_INTERVALS: i64 = 48;
