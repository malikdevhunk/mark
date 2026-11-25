#!/usr/bin/env node
/**
 * Print a devnet transaction's program logs.
 *   node scripts/verify_tx.mjs <signature>
 */
const sig = process.argv[2];
if (!sig) {
  console.error("usage: node scripts/verify_tx.mjs <signature>");
  process.exit(1);
}
const rpc = process.env.RPC_URL ?? "https://api.devnet.solana.com";
const body = {
  jsonrpc: "2.0",
  id: 1,
  method: "getTransaction",
  params: [sig, { encoding: "json", commitment: "confirmed", maxSupportedTransactionVersion: 0 }],
};
const res = await fetch(rpc, {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify(body),
});
const json = await res.json();
if (!json.result) {
  console.error(json.error ?? "transaction not found");
  process.exit(1);
}
const logs = json.result.meta?.logMessages ?? [];
for (const line of logs) console.log(line);
console.log("slot", json.result.slot);
