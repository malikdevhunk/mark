"use client";

import {
  BaseMessageSignerWalletAdapter,
  isVersionedTransaction,
  WalletNotConnectedError,
  WalletReadyState,
  type WalletName,
} from "@solana/wallet-adapter-base";
import { Keypair, type Transaction, type VersionedTransaction } from "@solana/web3.js";

export const DemoWalletName = "Demo wallet" as WalletName<"Demo wallet">;
const STORAGE = "mark-demo-secret";

function loadKeypair(): Keypair {
  const raw = window.localStorage.getItem(STORAGE);
  if (raw) {
    return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(raw) as number[]));
  }
  const created = Keypair.generate();
  window.localStorage.setItem(STORAGE, JSON.stringify(Array.from(created.secretKey)));
  return created;
}

/** A devnet-only key kept in this browser. It is not a vault. */
export class DemoWalletAdapter extends BaseMessageSignerWalletAdapter {
  name = DemoWalletName;
  url = "https://saddammalik.com";
  icon =
    "data:image/svg+xml;utf8," +
    encodeURIComponent(
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" rx="4" fill="#d7c4a3"/><text x="16" y="21" text-anchor="middle" font-family="sans-serif" font-size="14" fill="#1c1915">M</text></svg>`,
    );
  supportedTransactionVersions = new Set(["legacy", 0] as const);
  private keypair: Keypair | null = null;

  get connecting() {
    return false;
  }
  get publicKey() {
    return this.keypair?.publicKey ?? null;
  }
  get readyState() {
    return typeof window === "undefined" ? WalletReadyState.Unsupported : WalletReadyState.Loadable;
  }

  async connect() {
    this.keypair = loadKeypair();
    this.emit("connect", this.keypair.publicKey);
  }
  async disconnect() {
    this.keypair = null;
    this.emit("disconnect");
  }
  async signTransaction<T extends Transaction | VersionedTransaction>(transaction: T): Promise<T> {
    if (!this.keypair) throw new WalletNotConnectedError();
    if (isVersionedTransaction(transaction)) transaction.sign([this.keypair]);
    else transaction.partialSign(this.keypair);
    return transaction;
  }
  async signMessage(message: Uint8Array): Promise<Uint8Array> {
    if (!this.keypair) throw new WalletNotConnectedError();
    const { ed25519 } = await import("@noble/curves/ed25519.js");
    return ed25519.sign(message, this.keypair.secretKey.slice(0, 32));
  }
}
