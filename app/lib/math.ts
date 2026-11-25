/** Integer math shared with programs/mark_perp/src/math.rs. Prices are micro-units of probability. */

export const PRICE = 1_000_000n;
export const SIZE = 1_000_000n;
export const TICK = 1_000n;
export const BPS = 10_000n;

export const SKEW_COEFF = 1_000n;
export const SKEW_DIVISOR = 1_000_000n;
export const IMPACT_COEFF = 500n;
export const IMPACT_DIVISOR = 1_000_000n;
export const IM_BPS = 500n;
export const IM_HAIRCUT = 5_000n;
export const MM_BPS = 250n;
export const MM_HAIRCUT = 2_500n;
export const MAX_LEVERAGE = 20n;
export const FUNDING_DIVISOR = 10n;
export const MAX_PREMIUM = 50_000n;
export const FEE_BPS = 20n;
export const LIQ_BPS = 100n;

export function clamp(raw: bigint): bigint {
  if (raw < TICK) return TICK;
  if (raw > PRICE - TICK) return PRICE - TICK;
  return raw;
}

export function markOf(index: bigint, oiLong: bigint, oiShort: bigint): bigint {
  const diff = oiLong - oiShort;
  const skew = (diff * SKEW_COEFF) / SKEW_DIVISOR;
  return clamp(index + skew);
}

export function fillOf(mark: bigint, long: boolean, size: bigint): bigint {
  const impact = (size * IMPACT_COEFF) / IMPACT_DIVISOR;
  return clamp(long ? mark + impact : mark - impact);
}

export function pnlOf(sizeSigned: bigint, entry: bigint, price: bigint): bigint {
  return (sizeSigned * (price - entry)) / PRICE;
}

export function worstLoss(size: bigint, price: bigint, long: boolean): bigint {
  return long ? (size * price) / PRICE : (size * (PRICE - price)) / PRICE;
}

export function marginReq(
  size: bigint,
  price: bigint,
  long: boolean,
  floorBps: bigint,
  haircutBps: bigint,
  maxLev: bigint,
): bigint {
  if (size <= 0n) return 0n;
  const floor = (size * floorBps) / BPS;
  const hair = (worstLoss(size, price, long) * haircutBps) / BPS;
  const lev = maxLev === 0n ? 0n : size / maxLev;
  return floor > hair ? (floor > lev ? floor : lev) : hair > lev ? hair : lev;
}

export function initialMargin(size: bigint, entry: bigint, long: boolean): bigint {
  return marginReq(size, entry, long, IM_BPS, IM_HAIRCUT, MAX_LEVERAGE);
}

export function maintenance(size: bigint, price: bigint, long: boolean): bigint {
  return marginReq(size, price, long, MM_BPS, MM_HAIRCUT, 0n);
}

export function fundingPremium(mark: bigint, index: bigint): bigint {
  const raw = ((mark - index) * 1n) / FUNDING_DIVISOR;
  if (raw > MAX_PREMIUM) return MAX_PREMIUM;
  if (raw < -MAX_PREMIUM) return -MAX_PREMIUM;
  return raw;
}

export function fundingPayment(sizeSigned: bigint, premium: bigint): bigint {
  return (sizeSigned * premium) / SIZE;
}

export function takerFee(size: bigint): bigint {
  return (size * FEE_BPS) / BPS;
}

function liquidatable(
  sizeSigned: bigint,
  entry: bigint,
  margin: bigint,
  price: bigint,
): boolean {
  const equity = margin + pnlOf(sizeSigned, entry, price);
  const mm = maintenance(sizeSigned < 0n ? -sizeSigned : sizeSigned, price, sizeSigned > 0n);
  return equity < mm;
}

/** Highest surviving-failure price for a long, lowest for a short. */
export function liquidationPrice(
  sizeSigned: bigint,
  entry: bigint,
  margin: bigint,
): bigint | null {
  if (sizeSigned === 0n) return null;
  const long = sizeSigned > 0n;
  if (long) {
    if (!liquidatable(sizeSigned, entry, margin, TICK)) return null;
    let lo = TICK;
    let hi = PRICE - TICK;
    while (lo < hi) {
      const mid = lo + (hi - lo + 1n) / 2n;
      if (liquidatable(sizeSigned, entry, margin, mid)) lo = mid;
      else hi = mid - 1n;
    }
    return lo;
  }
  if (!liquidatable(sizeSigned, entry, margin, PRICE - TICK)) return null;
  let lo = TICK;
  let hi = PRICE - TICK;
  while (lo < hi) {
    const mid = lo + (hi - lo) / 2n;
    if (liquidatable(sizeSigned, entry, margin, mid)) hi = mid;
    else lo = mid + 1n;
  }
  return lo;
}

export type Quote = {
  size: bigint;
  fill: bigint;
  margin: bigint;
  fee: bigint;
  liq: bigint | null;
  leverage: number;
};

/** Size a market order so initial margin is about `riskMicro` USDC. */
export function quoteFromRisk(
  riskMicro: bigint,
  long: boolean,
  index: bigint,
  oiLong: bigint,
  oiShort: bigint,
): Quote {
  const mark = markOf(index, oiLong, oiShort);
  let size = SIZE;
  for (let i = 0; i < 4; i++) {
    const fill = fillOf(mark, long, size);
    const per = initialMargin(SIZE, fill, long);
    if (per <= 0n) break;
    const next = (riskMicro * SIZE) / per;
    size = next < 1n ? 1n : next;
  }
  const fill = fillOf(mark, long, size);
  const margin = initialMargin(size, fill, long);
  const signed = long ? size : -size;
  const liq = liquidationPrice(signed, fill, margin);
  const leverage = margin === 0n ? 0 : Number(size) / Number(margin);
  return { size, fill, margin, fee: takerFee(size), liq, leverage };
}

export function dollars(micro: bigint): string {
  const sign = micro < 0n ? "-" : "";
  const v = micro < 0n ? -micro : micro;
  const whole = v / 1_000_000n;
  const frac = (v % 1_000_000n).toString().padStart(6, "0").slice(0, 2);
  return `${sign}$${whole.toString()}.${frac}`;
}

export function cents(price: bigint): string {
  const scaled = (price * 1000n) / PRICE;
  const whole = scaled / 10n;
  const tenth = scaled % 10n;
  return `${whole.toString()}.${tenth.toString()}¢`;
}

export function contracts(size: bigint): string {
  const whole = size / SIZE;
  const frac = (size % SIZE).toString().padStart(6, "0").replace(/0+$/, "");
  return frac ? `${whole.toString()}.${frac}` : whole.toString();
}
