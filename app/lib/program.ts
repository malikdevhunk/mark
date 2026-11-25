import { sha256 } from "@noble/hashes/sha2.js";
import {
  Connection,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  type AccountMeta,
} from "@solana/web3.js";

export const PROGRAM_ID = new PublicKey(
  process.env.NEXT_PUBLIC_PROGRAM_ID ?? "EwCuCZyudnEjbHZ6wFYZtZBnpJaGEYsX2X8VJiMh5MrG",
);

export const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL ?? "https://api.devnet.solana.com";

const TOKEN = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ATA = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

function sighash(name: string): Buffer {
  return Buffer.from(sha256(new TextEncoder().encode(`global:${name}`))).subarray(0, 8);
}

function ix(name: string, body: Buffer, keys: AccountMeta[]): TransactionInstruction {
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys,
    data: Buffer.concat([sighash(name), body]),
  });
}

export function pda(seeds: Array<Buffer | Uint8Array>): PublicKey {
  return PublicKey.findProgramAddressSync(seeds, PROGRAM_ID)[0];
}

export function exchangePda(): PublicKey {
  return pda([Buffer.from("exchange")]);
}
export function vaultPda(): PublicKey {
  return pda([Buffer.from("vault")]);
}
export function insurancePda(): PublicKey {
  return pda([Buffer.from("insurance")]);
}
export function feePda(): PublicKey {
  return pda([Buffer.from("fees")]);
}
export function marketPda(id: number): PublicKey {
  const buf = Buffer.alloc(2);
  buf.writeUInt16LE(id);
  return pda([Buffer.from("market"), buf]);
}
export function userPda(owner: PublicKey): PublicKey {
  return pda([Buffer.from("user"), owner.toBuffer()]);
}
export function positionPda(market: PublicKey, owner: PublicKey): PublicKey {
  return pda([Buffer.from("position"), market.toBuffer(), owner.toBuffer()]);
}
export function ata(owner: PublicKey, mint: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [owner.toBuffer(), TOKEN.toBuffer(), mint.toBuffer()],
    ATA,
  )[0];
}

function u64(n: bigint): Buffer {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(n);
  return b;
}

function text(bytes: Uint8Array): string {
  const end = bytes.indexOf(0);
  const slice = end === -1 ? bytes : bytes.subarray(0, end);
  return new TextDecoder().decode(slice).trim();
}

function u64at(data: Buffer, at: number): bigint {
  return data.readBigUInt64LE(at);
}
function i64at(data: Buffer, at: number): bigint {
  return data.readBigInt64LE(at);
}

export type MarketView = {
  pubkey: PublicKey;
  id: number;
  symbol: string;
  question: string;
  resolution: string;
  oracleKind: number;
  strike: bigint;
  expiry: bigint;
  status: number;
  index: bigint;
  mark: bigint;
  conf: bigint;
  spot: bigint;
  oracleTs: bigint;
  lastFundingTs: bigint;
  lastPremium: bigint;
  oiLong: bigint;
  oiShort: bigint;
  maxAge: number;
  settled: bigint;
};

export function decodeMarket(pubkey: PublicKey, data: Buffer): MarketView {
  return {
    pubkey,
    id: data.readUInt16LE(8),
    symbol: text(data.subarray(10, 26)),
    question: text(data.subarray(26, 218)),
    resolution: text(data.subarray(218, 538)),
    oracleKind: data[538],
    strike: u64at(data, 571),
    expiry: i64at(data, 579),
    status: data[587],
    index: u64at(data, 588),
    mark: u64at(data, 596),
    conf: u64at(data, 604),
    spot: u64at(data, 612),
    oracleTs: i64at(data, 620),
    lastFundingTs: i64at(data, 629),
    lastPremium: i64at(data, 653),
    oiLong: u64at(data, 661),
    oiShort: u64at(data, 669),
    maxAge: data.readUInt32LE(773),
    settled: u64at(data, 779),
  };
}

export function decodeFree(data: Buffer): bigint {
  return u64at(data, 40);
}

export type PositionView = {
  size: bigint;
  entry: bigint;
  margin: bigint;
};

export function decodePosition(data: Buffer): PositionView {
  return {
    size: i64at(data, 72),
    entry: u64at(data, 80),
    margin: u64at(data, 88),
  };
}

export function initUserIx(owner: PublicKey): TransactionInstruction {
  return ix("initialize_user", Buffer.alloc(0), [
    { pubkey: owner, isSigner: true, isWritable: true },
    { pubkey: userPda(owner), isSigner: false, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ]);
}

export function faucetIx(owner: PublicKey, mint: PublicKey, amount: bigint): TransactionInstruction {
  return ix("faucet", u64(amount), [
    { pubkey: owner, isSigner: true, isWritable: true },
    { pubkey: exchangePda(), isSigner: false, isWritable: false },
    { pubkey: userPda(owner), isSigner: false, isWritable: true },
    { pubkey: mint, isSigner: false, isWritable: true },
    { pubkey: ata(owner, mint), isSigner: false, isWritable: true },
    { pubkey: TOKEN, isSigner: false, isWritable: false },
    { pubkey: ATA, isSigner: false, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ]);
}

export function depositIx(owner: PublicKey, mint: PublicKey, amount: bigint): TransactionInstruction {
  return ix("deposit_collateral", u64(amount), [
    { pubkey: owner, isSigner: true, isWritable: true },
    { pubkey: exchangePda(), isSigner: false, isWritable: false },
    { pubkey: userPda(owner), isSigner: false, isWritable: true },
    { pubkey: ata(owner, mint), isSigner: false, isWritable: true },
    { pubkey: vaultPda(), isSigner: false, isWritable: true },
    { pubkey: mint, isSigner: false, isWritable: false },
    { pubkey: TOKEN, isSigner: false, isWritable: false },
  ]);
}

export function openIx(
  owner: PublicKey,
  market: PublicKey,
  side: number,
  size: bigint,
  limit: bigint,
): TransactionInstruction {
  const body = Buffer.concat([Buffer.from([side]), u64(size), u64(limit)]);
  return ix("open_or_increase", body, [
    { pubkey: owner, isSigner: true, isWritable: true },
    { pubkey: exchangePda(), isSigner: false, isWritable: true },
    { pubkey: market, isSigner: false, isWritable: true },
    { pubkey: userPda(owner), isSigner: false, isWritable: true },
    { pubkey: positionPda(market, owner), isSigner: false, isWritable: true },
    { pubkey: vaultPda(), isSigner: false, isWritable: true },
    { pubkey: insurancePda(), isSigner: false, isWritable: true },
    { pubkey: feePda(), isSigner: false, isWritable: true },
    { pubkey: TOKEN, isSigner: false, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ]);
}

export function closeIx(
  owner: PublicKey,
  market: PublicKey,
  size: bigint,
  limit: bigint,
): TransactionInstruction {
  return ix("reduce_or_close", Buffer.concat([u64(size), u64(limit)]), [
    { pubkey: owner, isSigner: true, isWritable: true },
    { pubkey: exchangePda(), isSigner: false, isWritable: true },
    { pubkey: market, isSigner: false, isWritable: true },
    { pubkey: userPda(owner), isSigner: false, isWritable: true },
    { pubkey: positionPda(market, owner), isSigner: false, isWritable: true },
    { pubkey: vaultPda(), isSigner: false, isWritable: true },
    { pubkey: insurancePda(), isSigner: false, isWritable: true },
    { pubkey: feePda(), isSigner: false, isWritable: true },
    { pubkey: TOKEN, isSigner: false, isWritable: false },
  ]);
}

function pushStr(parts: Buffer[], value: string) {
  const bytes = Buffer.from(value, "utf8");
  const len = Buffer.alloc(4);
  len.writeUInt32LE(bytes.length);
  parts.push(len, bytes);
}

export function initializeExchangeIx(
  admin: PublicKey,
  mint: PublicKey,
  params: { perTx: bigint; perUser: bigint; insuranceTarget: bigint; drainCap: bigint },
): TransactionInstruction {
  const body = Buffer.concat([
    Buffer.from([1]),
    u64(params.perTx),
    u64(params.perUser),
    u64(params.insuranceTarget),
    u64(params.drainCap),
  ]);
  return ix("initialize_exchange", body, [
    { pubkey: admin, isSigner: true, isWritable: true },
    { pubkey: exchangePda(), isSigner: false, isWritable: true },
    { pubkey: mint, isSigner: true, isWritable: true },
    { pubkey: vaultPda(), isSigner: false, isWritable: true },
    { pubkey: insurancePda(), isSigner: false, isWritable: true },
    { pubkey: feePda(), isSigner: false, isWritable: true },
    { pubkey: TOKEN, isSigner: false, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ]);
}

export function seedInsuranceIx(admin: PublicKey, mint: PublicKey, amount: bigint): TransactionInstruction {
  return ix("seed_insurance", u64(amount), [
    { pubkey: admin, isSigner: true, isWritable: true },
    { pubkey: exchangePda(), isSigner: false, isWritable: true },
    { pubkey: mint, isSigner: false, isWritable: true },
    { pubkey: insurancePda(), isSigner: false, isWritable: true },
    { pubkey: TOKEN, isSigner: false, isWritable: false },
  ]);
}

export type InitMarket = {
  id: number;
  symbol: string;
  question: string;
  resolution: string;
  oracleKind: number;
  feed: Buffer;
  strike: bigint;
  expiry: bigint;
  index: bigint;
  spot: bigint;
  maxAge: number;
};

export function initializeMarketIx(admin: PublicKey, market: InitMarket): TransactionInstruction {
  const parts: Buffer[] = [];
  const id = Buffer.alloc(2);
  id.writeUInt16LE(market.id);
  parts.push(id);
  pushStr(parts, market.symbol);
  pushStr(parts, market.question);
  pushStr(parts, market.resolution);
  parts.push(Buffer.from([market.oracleKind]));
  parts.push(market.feed);
  parts.push(u64(market.strike), i64(market.expiry), u64(market.index), u64(0n), u64(market.spot));
  for (const n of [1_000n, 1_000_000n, 500n, 1_000_000n]) parts.push(u64(n));
  for (const n of [500, 5_000, 250, 2_500, 20]) parts.push(u16(n));
  for (const n of [1n, 10n, 50_000n]) parts.push(u64(n));
  parts.push(i64(3600n));
  parts.push(u64(1_000_000n * 1_000_000n), u64(100_000n * 1_000_000n));
  for (const n of [20, 100, 2_000]) parts.push(u16(n));
  const age = Buffer.alloc(4);
  age.writeUInt32LE(market.maxAge);
  parts.push(age, u16(250));
  return ix("initialize_market", Buffer.concat(parts), [
    { pubkey: admin, isSigner: true, isWritable: true },
    { pubkey: exchangePda(), isSigner: false, isWritable: true },
    { pubkey: marketPda(market.id), isSigner: false, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ]);
}

function u16(n: number): Buffer {
  const b = Buffer.alloc(2);
  b.writeUInt16LE(n);
  return b;
}

function i64(n: bigint): Buffer {
  const b = Buffer.alloc(8);
  b.writeBigInt64LE(n);
  return b;
}

export function pushIndexIx(admin: PublicKey, market: PublicKey, index: bigint, spot: bigint): TransactionInstruction {
  return ix("push_admin_index", Buffer.concat([u64(index), u64(0n), u64(spot)]), [
    { pubkey: admin, isSigner: true, isWritable: true },
    { pubkey: exchangePda(), isSigner: false, isWritable: false },
    { pubkey: market, isSigner: false, isWritable: true },
  ]);
}

export function fundingIx(market: PublicKey): TransactionInstruction {
  return ix("update_funding", Buffer.alloc(0), [
    { pubkey: market, isSigner: false, isWritable: true },
  ]);
}

export async function sendIxs(
  connection: Connection,
  owner: PublicKey,
  sign: (tx: Transaction) => Promise<Transaction>,
  ixs: TransactionInstruction[],
): Promise<string> {
  const tx = new Transaction().add(...ixs);
  tx.feePayer = owner;
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = blockhash;
  const signed = await sign(tx);
  const sig = await connection.sendRawTransaction(signed.serialize(), { skipPreflight: false });
  await connection.confirmTransaction({ signature: sig, blockhash, lastValidBlockHeight }, "confirmed");
  return sig;
}

export function solscan(sig: string): string {
  return `https://solscan.io/tx/${sig}?cluster=devnet`;
}

export function plainError(err: unknown): string {
  const text = err instanceof Error ? err.message : String(err);
  const lower = text.toLowerCase();
  if (lower.includes("stale")) return "Oracle is stale. New trades are blocked until a fresh print lands.";
  if (lower.includes("paused") || lower.includes("not taking")) return "Trading is paused. Closes are still allowed.";
  if (lower.includes("free collateral") || lower.includes("insufficient"))
    return "Free collateral is too low for this size.";
  if (lower.includes("user rejected") || lower.includes("rejected")) return "The wallet declined the signature.";
  if (lower.includes("blockhash")) return "The network dropped the blockhash. Try the signature again.";
  const line = text.split("\n").find((l) => l.includes("Error Message"));
  return line ? line.replace("Error Message:", "").trim() : "The transaction did not land.";
}
