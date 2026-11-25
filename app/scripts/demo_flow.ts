/**
 * Devnet rehearsal: airdrop, faucet, deposit, open, funding crank, close.
 * cd app && npx tsx ../scripts/demo_flow.ts
 */
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { Connection, Keypair } from "@solana/web3.js";
import {
  closeIx,
  depositIx,
  exchangePda,
  faucetIx,
  fundingIx,
  initUserIx,
  marketPda,
  openIx,
  PROGRAM_ID,
  sendIxs,
  solscan,
} from "../lib/program";

const RPC = process.env.RPC_URL ?? "https://api.devnet.solana.com";

function load(path: string): Keypair {
  return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(path, "utf8"))));
}

async function main() {
  const traderPath = "../keys/demo.json";
  const trader = existsSync(traderPath)
    ? load(traderPath)
    : Keypair.generate();
  if (!existsSync(traderPath)) {
    writeFileSync(traderPath, JSON.stringify(Array.from(trader.secretKey)));
  }
  const connection = new Connection(RPC, "confirmed");
  const exchange = await connection.getAccountInfo(exchangePda());
  if (!exchange) throw new Error("Exchange is not initialised. Run scripts/init_markets.ts");
  const mint = new (await import("@solana/web3.js")).PublicKey(exchange.data.subarray(40, 72));
  const market = marketPda(0);

  try {
    const sig = await connection.requestAirdrop(trader.publicKey, 1_000_000_000);
    await connection.confirmTransaction(sig, "confirmed");
  } catch {
    console.log("SOL airdrop skipped");
  }

  const sign = async (tx: import("@solana/web3.js").Transaction) => {
    tx.partialSign(trader);
    return tx;
  };
  const amount = 20_000_000n;
  const user = await connection.getAccountInfo(
    (await import("../lib/program")).userPda(trader.publicKey),
  );
  const setup = [];
  if (!user) setup.push(initUserIx(trader.publicKey));
  setup.push(faucetIx(trader.publicKey, mint, amount), depositIx(trader.publicKey, mint, amount));
  const funded = await sendIxs(connection, trader.publicKey, sign, setup);
  console.log("deposit", solscan(funded));

  const opened = await sendIxs(connection, trader.publicKey, sign, [
    openIx(trader.publicKey, market, 0, 1_000_000n, 1_000_000n),
  ]);
  console.log("open", solscan(opened));

  try {
    const crank = await sendIxs(connection, trader.publicKey, sign, [fundingIx(market)]);
    console.log("funding", solscan(crank));
  } catch (err) {
    console.log("funding skipped until the hour elapses:", err instanceof Error ? err.message.split("\n")[0] : err);
  }

  const closed = await sendIxs(connection, trader.publicKey, sign, [
    closeIx(trader.publicKey, market, 1_000_000n, 0n),
  ]);
  console.log("close", solscan(closed));
  console.log("program", PROGRAM_ID.toBase58());
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
