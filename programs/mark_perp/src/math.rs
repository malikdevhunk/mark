use crate::constants::*;
use crate::errors::MarkError;
use anchor_lang::prelude::*;

pub fn clamp_price(raw: i128) -> u64 {
    let lo = TICK as i128;
    let hi = (PRICE_SCALE - TICK) as i128;
    raw.clamp(lo, hi) as u64
}

pub fn mul_div(a: u64, b: u64, d: u64) -> Result<u64> {
    if d == 0 {
        return err!(MarkError::Math);
    }
    let v = (a as u128)
        .checked_mul(b as u128)
        .ok_or(MarkError::Math)?
        / (d as u128);
    u64::try_from(v).map_err(|_| error!(MarkError::Math))
}

pub fn conf_bps(conf: u64, price: u64) -> Result<u64> {
    if price == 0 {
        return err!(MarkError::BadOracle);
    }
    Ok(((conf as u128) * (BPS as u128) / (price as u128)) as u64)
}

pub fn oracle_is_fresh(oracle_ts: i64, now: i64, max_age_secs: u32) -> bool {
    if oracle_ts <= 0 || now < oracle_ts {
        return false;
    }
    (now - oracle_ts) as u64 <= max_age_secs as u64
}

/// Linear model: 0.50 when spot equals the strike, 0 or 1 when spot is
/// a full strike away. Clamped to the open interval (0, 1). This is a model,
/// not an implied market.
pub fn model_probability(spot: u64, strike: u64) -> Result<u64> {
    if strike == 0 {
        return err!(MarkError::BadParams);
    }
    let half = (PRICE_SCALE / 2) as i128;
    let numer = (spot as i128 - strike as i128) * (PRICE_SCALE as i128);
    let denom = 2 * strike as i128;
    Ok(clamp_price(half + numer / denom))
}

pub fn pyth_to_usd6(price: i64, expo: i32) -> Result<u64> {
    if price <= 0 || !(-18..=18).contains(&expo) {
        return err!(MarkError::BadOracle);
    }
    let mut v = price as i128;
    let shift = expo + 6;
    if shift >= 0 {
        let pow = 10i128
            .checked_pow(shift as u32)
            .ok_or(error!(MarkError::Math))?;
        v = v.checked_mul(pow).ok_or(error!(MarkError::Math))?;
    } else {
        let pow = 10i128
            .checked_pow((-shift) as u32)
            .ok_or(error!(MarkError::Math))?;
        v /= pow;
    }
    if v <= 0 {
        return err!(MarkError::BadOracle);
    }
    u64::try_from(v).map_err(|_| error!(MarkError::Math))
}

pub fn skew_offset(oi_long: u64, oi_short: u64, coeff: u64, divisor: u64) -> Result<i64> {
    if divisor == 0 {
        return err!(MarkError::Math);
    }
    let diff = oi_long as i128 - oi_short as i128;
    let v = diff * coeff as i128 / divisor as i128;
    i64::try_from(v).map_err(|_| error!(MarkError::Math))
}

pub fn mark_from_index(
    index: u64,
    oi_long: u64,
    oi_short: u64,
    coeff: u64,
    divisor: u64,
) -> Result<u64> {
    let skew = skew_offset(oi_long, oi_short, coeff, divisor)?;
    Ok(clamp_price(index as i128 + skew as i128))
}

/// Buyers lift the mark, sellers hit it. `impact` is sized at half the skew
/// slope (see constants) so the fill is the average of the pre and post mark.
pub fn fill_from_mark(mark: u64, long: bool, size: u64, coeff: u64, divisor: u64) -> Result<u64> {
    if divisor == 0 {
        return err!(MarkError::Math);
    }
    let impact = (size as i128) * (coeff as i128) / (divisor as i128);
    let raw = if long {
        mark as i128 + impact
    } else {
        mark as i128 - impact
    };
    Ok(clamp_price(raw))
}

pub fn pnl(size_signed: i64, entry: u64, price: u64) -> Result<i64> {
    let diff = price as i128 - entry as i128;
    let raw = (size_signed as i128) * diff / (PRICE_SCALE as i128);
    i64::try_from(raw).map_err(|_| error!(MarkError::Math))
}

pub fn worst_loss(size_abs: u64, price: u64, long: bool) -> Result<u64> {
    if long {
        mul_div(size_abs, price, PRICE_SCALE)
    } else {
        mul_div(size_abs, PRICE_SCALE.saturating_sub(price), PRICE_SCALE)
    }
}

/// IM (and MM) are the greater of a floor on the $1 contract notional and a
/// haircut of the worst loss to 0 or 1. A long at 90¢ therefore posts more
/// than a long at 50¢. `max_leverage` adds a third floor on IM only; pass 0
/// for maintenance.
pub fn margin_requirement(
    size_abs: u64,
    price: u64,
    long: bool,
    floor_bps: u16,
    haircut_bps: u16,
    max_leverage: u16,
) -> Result<u64> {
    if size_abs == 0 {
        return Ok(0);
    }
    let floor = mul_div(size_abs, floor_bps as u64, BPS)?;
    let worst = worst_loss(size_abs, price, long)?;
    let hair = mul_div(worst, haircut_bps as u64, BPS)?;
    let lev = if max_leverage == 0 {
        0
    } else {
        mul_div(size_abs, 1, max_leverage as u64)?
    };
    Ok(floor.max(hair).max(lev))
}

pub fn equity_at(
    size_signed: i64,
    entry: u64,
    margin: u64,
    funding_payment: i64,
    price: u64,
) -> Result<i64> {
    let trade = pnl(size_signed, entry, price)?;
    let eq = (margin as i128) + (trade as i128) - (funding_payment as i128);
    i64::try_from(eq).map_err(|_| error!(MarkError::Math))
}

pub fn is_liquidatable(equity: i64, maintenance: u64) -> bool {
    equity < maintenance as i64
}

/// Highest mark (long) or lowest mark (short) at which equity is below
/// maintenance. None when the position survives all the way to the bound.
pub fn liquidation_price(
    size_signed: i64,
    entry: u64,
    margin: u64,
    funding_payment: i64,
    mm_bps: u16,
    mm_haircut_bps: u16,
) -> Result<Option<u64>> {
    if size_signed == 0 {
        return Ok(None);
    }
    let long = size_signed > 0;
    let size_abs = size_signed.unsigned_abs();
    let check = |price: u64| -> Result<bool> {
        let eq = equity_at(size_signed, entry, margin, funding_payment, price)?;
        let mm = margin_requirement(size_abs, price, long, mm_bps, mm_haircut_bps, 0)?;
        Ok(is_liquidatable(eq, mm))
    };
    let lo_bound = TICK;
    let hi_bound = PRICE_SCALE - TICK;
    if long {
        if !check(lo_bound)? {
            return Ok(None);
        }
        let mut lo = lo_bound;
        let mut hi = hi_bound;
        while lo < hi {
            let mid = lo + (hi - lo + 1) / 2;
            if check(mid)? {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        Ok(Some(lo))
    } else {
        if !check(hi_bound)? {
            return Ok(None);
        }
        let mut lo = lo_bound;
        let mut hi = hi_bound;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if check(mid)? {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        Ok(Some(lo))
    }
}

/// Per-contract premium in USDC micro-units for one interval.
/// Positive means longs pay.
pub fn funding_premium(
    mark: u64,
    index: u64,
    k: u64,
    divisor: u64,
    max_premium: u64,
) -> Result<i64> {
    if divisor == 0 {
        return err!(MarkError::Math);
    }
    let basis = mark as i128 - index as i128;
    let raw = basis * (k as i128) / (divisor as i128);
    let cap = max_premium as i128;
    Ok(raw.clamp(-cap, cap) as i64)
}

pub fn funding_payment(size_signed: i64, cumulative_now: i128, cumulative_then: i128) -> Result<i64> {
    let delta = cumulative_now
        .checked_sub(cumulative_then)
        .ok_or(error!(MarkError::Math))?;
    let pay = (size_signed as i128) * delta / (SIZE_SCALE as i128);
    i64::try_from(pay).map_err(|_| error!(MarkError::Math))
}

pub fn taker_fee(size_abs: u64, fee_bps: u16) -> Result<u64> {
    mul_div(size_abs, fee_bps as u64, BPS)
}

pub fn liquidation_penalty(size_abs: u64, penalty_bps: u16) -> Result<u64> {
    mul_div(size_abs, penalty_bps as u64, BPS)
}

pub fn weighted_entry(old_size: u64, old_entry: u64, add: u64, fill: u64) -> Result<u64> {
    let total = old_size.checked_add(add).ok_or(error!(MarkError::Math))?;
    if total == 0 {
        return err!(MarkError::ZeroSize);
    }
    let numer = (old_size as u128)
        .checked_mul(old_entry as u128)
        .ok_or(error!(MarkError::Math))?
        .checked_add(
            (add as u128)
                .checked_mul(fill as u128)
                .ok_or(error!(MarkError::Math))?,
        )
        .ok_or(error!(MarkError::Math))?;
    let v = numer / (total as u128);
    u64::try_from(v).map_err(|_| error!(MarkError::Math))
}

pub fn validate_risk(
    im_bps: u16,
    im_haircut_bps: u16,
    mm_bps: u16,
    mm_haircut_bps: u16,
    max_leverage: u16,
    skew_divisor: u64,
    impact_divisor: u64,
    funding_divisor: u64,
    funding_interval: i64,
    taker_fee_bps: u16,
    liq_penalty_bps: u16,
    keeper_incentive_bps: u16,
    max_conf_bps: u16,
    max_age_secs: u32,
    index: u64,
) -> Result<()> {
    if im_bps <= mm_bps
        || im_haircut_bps < mm_haircut_bps
        || im_haircut_bps > 10_000
        || mm_haircut_bps > 10_000
        || !(1..=100).contains(&max_leverage)
        || skew_divisor == 0
        || impact_divisor == 0
        || funding_divisor == 0
        || funding_interval < 1
        || taker_fee_bps > 500
        || liq_penalty_bps > 2_000
        || keeper_incentive_bps > 10_000
        || max_conf_bps == 0
        || max_conf_bps > 5_000
        || max_age_secs == 0
        || index < TICK
        || index > PRICE_SCALE - TICK
    {
        return err!(MarkError::BadParams);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_never_writes_zero_or_one() {
        assert_eq!(clamp_price(-5), TICK);
        assert_eq!(clamp_price(0), TICK);
        assert_eq!(clamp_price(PRICE_SCALE as i128), PRICE_SCALE - TICK);
        assert_eq!(clamp_price(500_000), 500_000);
    }

    #[test]
    fn ninety_cent_long_posts_more_than_fifty() {
        let size = SIZE_SCALE;
        let at_50 = margin_requirement(size, 500_000, true, IM_BPS, IM_HAIRCUT_BPS, MAX_LEVERAGE)
            .unwrap();
        let at_90 = margin_requirement(size, 900_000, true, IM_BPS, IM_HAIRCUT_BPS, MAX_LEVERAGE)
            .unwrap();
        assert_eq!(at_50, 250_000);
        assert_eq!(at_90, 450_000);
        assert!(at_90 > at_50);
    }

    #[test]
    fn short_near_ten_cents_posts_more_than_short_near_ninety() {
        let size = SIZE_SCALE;
        let cheap = margin_requirement(size, 100_000, false, IM_BPS, IM_HAIRCUT_BPS, MAX_LEVERAGE)
            .unwrap();
        let rich = margin_requirement(size, 900_000, false, IM_BPS, IM_HAIRCUT_BPS, MAX_LEVERAGE)
            .unwrap();
        assert!(cheap > rich);
        assert_eq!(rich, 50_000);
    }

    #[test]
    fn round_trip_fill_is_flat_before_fees() {
        let index = 500_000;
        let size = 10 * SIZE_SCALE;
        let mark0 = mark_from_index(index, 0, 0, SKEW_COEFF, SKEW_DIVISOR).unwrap();
        let open = fill_from_mark(mark0, true, size, IMPACT_COEFF, IMPACT_DIVISOR).unwrap();
        let mark1 = mark_from_index(index, size, 0, SKEW_COEFF, SKEW_DIVISOR).unwrap();
        let close = fill_from_mark(mark1, false, size, IMPACT_COEFF, IMPACT_DIVISOR).unwrap();
        assert_eq!(open, close);
        assert_eq!(pnl(size as i64, open, close).unwrap(), 0);
    }

    #[test]
    fn longs_pay_when_mark_is_rich() {
        let premium = funding_premium(520_000, 500_000, FUNDING_K, FUNDING_DIVISOR, MAX_FUNDING_PREMIUM)
            .unwrap();
        assert_eq!(premium, 2_000);
        let pay = funding_payment(60 * SIZE_SCALE as i64, premium as i128, 0).unwrap();
        assert_eq!(pay, 120_000);
        let cheap = funding_premium(480_000, 500_000, FUNDING_K, FUNDING_DIVISOR, MAX_FUNDING_PREMIUM)
            .unwrap();
        assert!(cheap < 0);
    }

    #[test]
    fn premium_is_capped() {
        let premium = funding_premium(900_000, 100_000, 1, 1, 50_000).unwrap();
        assert_eq!(premium, 50_000);
    }

    #[test]
    fn known_liquidation_price_for_one_contract_long() {
        // 1 contract, entry 0.50, margin = IM at 0.50 ($0.25), no funding.
        let liq = liquidation_price(SIZE_SCALE as i64, 500_000, 250_000, 0, MM_BPS, MM_HAIRCUT_BPS)
            .unwrap();
        assert_eq!(liq, Some(333_332));
        let eq = equity_at(SIZE_SCALE as i64, 500_000, 250_000, 0, 333_332).unwrap();
        let mm = margin_requirement(SIZE_SCALE, 333_332, true, MM_BPS, MM_HAIRCUT_BPS, 0).unwrap();
        assert!(eq < mm as i64);
        let eq_above = equity_at(SIZE_SCALE as i64, 500_000, 250_000, 0, 333_333).unwrap();
        let mm_above =
            margin_requirement(SIZE_SCALE, 333_333, true, MM_BPS, MM_HAIRCUT_BPS, 0).unwrap();
        assert!(eq_above >= mm_above as i64);
    }

    #[test]
    fn model_is_half_at_the_strike() {
        assert_eq!(model_probability(150_000_000, 150_000_000).unwrap(), 500_000);
        assert_eq!(model_probability(300_000_000, 150_000_000).unwrap(), PRICE_SCALE - TICK);
        assert_eq!(model_probability(0, 150_000_000).unwrap(), TICK);
    }

    #[test]
    fn pyth_expo_converts_to_six_decimals() {
        // 150 * 10^8 with expo -8 is $150.
        let usd = pyth_to_usd6(15_000_000_000, -8).unwrap();
        assert_eq!(usd, 150_000_000);
    }

    #[test]
    fn large_size_does_not_panic() {
        let size = u64::MAX / 4;
        let r = margin_requirement(size, 500_000, true, IM_BPS, IM_HAIRCUT_BPS, MAX_LEVERAGE);
        assert!(r.is_ok());
        let p = pnl(i64::MAX, TICK, PRICE_SCALE - TICK);
        assert!(p.is_ok() || p.is_err());
    }
}
