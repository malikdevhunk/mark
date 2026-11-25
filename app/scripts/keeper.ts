/**
 * One funding crank, and a fresh admin index for the event market.
 * cd app && npx tsx ../scripts/keeper.ts
 */
import { readFileSync } from "node:fs";
import { Connection, Keypair } from "@solana/web3.js";
import { exchangePda, fundingIx, marketPda, pushIndexIx, sendIxs } from "../lib/program";

const RPC = process.env.RPC_URL ?? "https://api.devnet.solana.com";

async function main() {
  const admin = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(readFileSync("../keys/admin.json", "utf8"))),
  );
  const connection = new Connection(RPC, "confirmed");
  const exchange = await connection.getAccountInfo(exchangePda());
  if (!exchange) throw new Error("Exchange missing");
  const count = exchange.data.readUInt16LE(186);
  const sign = async (tx: import("@solana/web3.js").Transaction) => {
    tx.partialSign(admin);
    return tx;
  };
  for (let id = 0; id < count; id++) {
    const market = marketPda(id);
    try {
      const sig = await sendIxs(connection, admin.publicKey, sign, [fundingIx(market)]);
      console.log("funding", id, sig);
    } catch (err) {
      console.log("funding skipped", id, err instanceof Error ? err.message.split("\n")[0] : err);
    }
  }
  if (count > 2) {
    const sig = await sendIxs(connection, admin.publicKey, sign, [
      pushIndexIx(admin.publicKey, marketPda(2), 850_000n, 0n),
    ]);
    console.log("event index", sig);
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
