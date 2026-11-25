/**
 * Create the exchange, seed insurance, and open the three demo markets.
 * Run after scripts/deploy.sh:  cd app && npx tsx ../scripts/init_markets.ts
 */
import { readFileSync } from "node:fs";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import {
  exchangePda,
  initializeExchangeIx,
  initializeMarketIx,
  marketPda,
  pushIndexIx,
  seedInsuranceIx,
  sendIxs,
  PROGRAM_ID,
} from "../lib/program";

const RPC = process.env.RPC_URL ?? "https://api.devnet.solana.com";
const SOL_FEED = "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
const BTC_FEED = "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43";

function feed(hex: string): Buffer {
  return Buffer.from(hex, "hex");
}

function fridayExpiry(now = new Date()): number {
  const d = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate(), 21, 0, 0));
  const day = d.getUTCDay();
  const add = (5 - day + 7) % 7 || 7;
  d.setUTCDate(d.getUTCDate() + add);
  return Math.floor(d.getTime() / 1000);
}

async function spot(id: string): Promise<bigint | null> {
  try {
    const url = `https://hermes.pyth.network/v2/updates/price/latest?ids[]=0x${id}`;
    const res = await fetch(url);
    if (!res.ok) return null;
    const body = (await res.json()) as {
      parsed?: { price?: { price?: string; expo?: number } }[];
    };
    const price = body.parsed?.[0]?.price;
    if (!price?.price || price.expo == null) return null;
    const value = BigInt(price.price);
    const shift = BigInt(price.expo + 6);
    const usd = shift >= 0n ? value * 10n ** shift : value / 10n ** -shift;
    return usd;
  } catch {
    return null;
  }
}

function model(spot6: bigint, strike6: bigint): bigint {
  const half = 500_000n;
  const raw = half + ((spot6 - strike6) * 1_000_000n) / (2n * strike6);
  if (raw < 1_000n) return 1_000n;
  if (raw > 999_000n) return 999_000n;
  return raw;
}

function roundStrike(spot6: bigint): bigint {
  const dollars = Number(spot6) / 1e6;
  const step = dollars > 10_000 ? 5_000 : dollars > 100 ? 10 : 1;
  const rounded = Math.round(dollars / step) * step;
  return BigInt(Math.max(1, rounded)) * 1_000_000n;
}

async function main() {
  const admin = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync("../keys/admin.json", "utf8"))));
  const connection = new Connection(RPC, "confirmed");
  const sign = async (tx: import("@solana/web3.js").Transaction) => {
    tx.partialSign(admin);
    return tx;
  };

  const existing = await connection.getAccountInfo(exchangePda());
  let mint: PublicKey;
  if (!existing) {
    const created = Keypair.generate();
    mint = created.publicKey;
    const txSign = async (tx: import("@solana/web3.js").Transaction) => {
      tx.partialSign(admin, created);
      return tx;
    };
    await sendIxs(connection, admin.publicKey, txSign, [
      initializeExchangeIx(admin.publicKey, mint, {
        perTx: 100_000_000n,
        perUser: 500_000_000n,
        insuranceTarget: 500_000_000n,
        drainCap: 1_000_000_000_000n,
      }),
    ]);
    await sendIxs(connection, admin.publicKey, sign, [seedInsuranceIx(admin.publicKey, mint, 500_000_000n)]);
    console.log("Exchange", exchangePda().toBase58());
    console.log("Mint", mint.toBase58());
  } else {
    mint = new PublicKey(existing.data.subarray(40, 72));
    console.log("Exchange already live", exchangePda().toBase58());
    console.log("Mint", mint.toBase58());
  }

  const expiry = BigInt(fridayExpiry());
  const solSpot = (await spot(SOL_FEED)) ?? 150_000_000n;
  const btcSpot = (await spot(BTC_FEED)) ?? 100_000_000_000n;
  const solStrike = roundStrike(solSpot);
  const btcStrike = roundStrike(btcSpot);
  const markets = [
    {
      id: 0,
      symbol: "SOL-ABOVE-WEEK",
      question: `Will SOL/USD finish Friday 21:00 UTC above $${Number(solStrike) / 1e6}?`,
      resolution:
        "Index is a model from the Pyth SOL/USD spot: 0.50 at the strike, clamped to 0.001–0.999. Settlement freezes that model at expiry. It is a model, not an implied market.",
      oracleKind: 1,
      feed: feed(SOL_FEED),
      strike: solStrike,
      expiry,
      index: model(solSpot, solStrike),
      spot: solSpot,
      maxAge: 21_600,
    },
    {
      id: 1,
      symbol: "BTC-ABOVE-WEEK",
      question: `Will BTC/USD finish Friday 21:00 UTC above $${Number(btcStrike) / 1e6}?`,
      resolution:
        "Index is a model from the Pyth BTC/USD spot: 0.50 at the strike, clamped to 0.001–0.999. Settlement freezes that model at expiry. It is a model, not an implied market.",
      oracleKind: 1,
      feed: feed(BTC_FEED),
      strike: btcStrike,
      expiry,
      index: model(btcSpot, btcStrike),
      spot: btcSpot,
      maxAge: 21_600,
    },
    {
      id: 2,
      symbol: "EVENT-ODDS",
      question: "Will mainnet-beta be listed operational at status.solana.com at Friday 21:00 UTC?",
      resolution:
        "Published reference, not a Pyth model. The admin posts the probability on the tape. At expiry the market settles to 0.999 if the status page lists operational, otherwise 0.001.",
      oracleKind: 0,
      feed: Buffer.alloc(32),
      strike: 0n,
      expiry,
      index: 850_000n,
      spot: 0n,
      maxAge: 604_800,
    },
  ];

  const count = existing ? existing.data.readUInt16LE(186) : 0;
  for (const market of markets) {
    if (market.id < count) {
      console.log("Market", market.id, "exists", marketPda(market.id).toBase58());
      continue;
    }
    await sendIxs(connection, admin.publicKey, sign, [initializeMarketIx(admin.publicKey, market)]);
    if (market.oracleKind === 0) {
      await sendIxs(connection, admin.publicKey, sign, [
        pushIndexIx(admin.publicKey, marketPda(market.id), market.index, market.spot),
      ]);
    }
    console.log(market.symbol, marketPda(market.id).toBase58());
  }
  console.log("Program", PROGRAM_ID.toBase58());
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
