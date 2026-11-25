"use client";

import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import { PublicKey } from "@solana/web3.js";
import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Rail } from "./Rail";
import { Term } from "./Term";
import { DemoWalletName } from "./DemoWallet";
import {
  cents,
  contracts,
  dollars,
  fillOf,
  fundingPayment,
  fundingPremium,
  markOf,
  pnlOf,
  quoteFromRisk,
  takerFee,
} from "@/lib/math";
import {
  closeIx,
  decodeFree,
  decodeMarket,
  decodePosition,
  depositIx,
  exchangePda,
  faucetIx,
  fundingIx,
  initUserIx,
  marketPda,
  openIx,
  plainError,
  positionPda,
  PROGRAM_ID,
  sendIxs,
  solscan,
  userPda,
  type MarketView,
  type PositionView,
} from "@/lib/program";

const EXAMPLE: MarketView = {
  pubkey: PROGRAM_ID,
  id: 0,
  symbol: "EXAMPLE",
  question: "Example: will the event finish above the listed strike?",
  resolution: "Example series. Live markets replace this once the program answers on devnet.",
  oracleKind: 0,
  strike: 0n,
  expiry: 0n,
  status: 0,
  index: 500_000n,
  mark: 502_000n,
  conf: 0n,
  spot: 0n,
  oracleTs: 0n,
  lastFundingTs: 0n,
  lastPremium: 2_000n,
  oiLong: 2_000_000n,
  oiShort: 0n,
  maxAge: 3600,
  settled: 0n,
};

type Receipt = {
  sig: string;
  entry: string;
  exit: string;
  funding: string;
  net: string;
};

type Point = { t: number; mark: number; index: number; example?: boolean };

export function Desk() {
  const { connection } = useConnection();
  const { publicKey, signTransaction, connected, select, connect, disconnect } = useWallet();
  const { setVisible } = useWalletModal();
  const [markets, setMarkets] = useState<MarketView[]>([]);
  const [live, setLive] = useState(false);
  const [mint, setMint] = useState<string | null>(null);
  const [selected, setSelected] = useState(0);
  const [free, setFree] = useState<bigint | null>(null);
  const [position, setPosition] = useState<PositionView | null>(null);
  const [side, setSide] = useState<"buy" | "sell">("buy");
  const [risk, setRisk] = useState("10");
  const [fair, setFair] = useState("55");
  const [tab, setTab] = useState<"position" | "funding" | "tape">("position");
  const [busy, setBusy] = useState("");
  const [note, setNote] = useState("This is a perpetual on the chance of an event. You are on Solana devnet.");
  const [receipt, setReceipt] = useState<Receipt | null>(null);
  const [points, setPoints] = useState<Point[]>([]);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  const [sol, setSol] = useState<number | null>(null);

  const market = markets[selected] ?? EXAMPLE;
  const example = !live;

  const refresh = useCallback(async () => {
    try {
      const exchange = exchangePda();
      const info = await connection.getAccountInfo(exchange);
      if (!info) {
        setLive(false);
        setMarkets([]);
        return;
      }
      const count = info.data.readUInt16LE(186);
      const mintKey = new PublicKey(info.data.subarray(40, 72));
      setMint(mintKey.toBase58());
      const keys = Array.from({ length: count }, (_, i) => marketPda(i));
      const accounts = keys.length ? await connection.getMultipleAccountsInfo(keys) : [];
      const decoded = accounts.flatMap((acc, i) => (acc ? [decodeMarket(keys[i], Buffer.from(acc.data))] : []));
      setMarkets(decoded);
      setLive(decoded.length > 0);
      if (publicKey) {
        const user = await connection.getAccountInfo(userPda(publicKey));
        setFree(user ? decodeFree(Buffer.from(user.data)) : null);
        const m = decoded[selected];
        if (m) {
          const pos = await connection.getAccountInfo(positionPda(m.pubkey, publicKey));
          setPosition(pos ? decodePosition(Buffer.from(pos.data)) : null);
        }
        setSol(await connection.getBalance(publicKey));
      }
    } catch {
      setLive(false);
    }
  }, [connection, publicKey, selected]);

  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refresh(), 8000);
    const clock = window.setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000);
    return () => {
      window.clearInterval(id);
      window.clearInterval(clock);
    };
  }, [refresh]);

  useEffect(() => {
    if (!live) {
      setPoints([
        { t: 0, mark: 0.5, index: 0.5, example: true },
        { t: 1, mark: 0.52, index: 0.5, example: true },
        { t: 2, mark: 0.51, index: 0.49, example: true },
        { t: 3, mark: 0.53, index: 0.5, example: true },
      ]);
      return;
    }
    setPoints((prev) => {
      const next = [
        ...prev.filter((p) => !p.example),
        { t: Date.now(), mark: Number(market.mark) / 1e6, index: Number(market.index) / 1e6 },
      ];
      return next.slice(-32);
    });
  }, [live, market.mark, market.index]);

  const long = side === "buy";
  const riskMicro = BigInt(Math.round((Number(risk) || 0) * 1_000_000));
  const fairPrice = BigInt(Math.round((Number(fair) || 0) * 10_000));
  const quote = useMemo(() => {
    if (riskMicro <= 0n) return null;
    return quoteFromRisk(riskMicro, long, market.index, market.oiLong, market.oiShort);
  }, [riskMicro, long, market.index, market.oiLong, market.oiShort]);

  const age = market.oracleTs === 0n ? null : now - Number(market.oracleTs);
  const stale = !example && age != null && age > market.maxAge;
  const paused = market.status === 1 || market.status === 2;
  const settled = market.status === 3;
  const mark = markOf(market.index, market.oiLong, market.oiShort);
  const premium = fundingPremium(mark, market.index);
  const basisCents = Number(mark - market.index) / 10_000;

  const sentences = useMemo(() => {
    if (!quote) return ["Set a risk amount to see the three outcomes.", "", ""];
    const signed = long ? quote.size : -quote.size;
    const up = pnlOf(signed, quote.fill, fairPrice < 1_000n ? 1_000n : fairPrice > 999_000n ? 999_000n : fairPrice);
    const against = long ? mark - 100_000n : mark + 100_000n;
    const clamped = against < 1_000n ? 1_000n : against > 999_000n ? 999_000n : against;
    const downPrice = quote.liq != null && ((long && clamped < quote.liq) || (!long && clamped > quote.liq)) ? quote.liq : clamped;
    const down = pnlOf(signed, quote.fill, downPrice);
    const liq = quote.liq == null
      ? "This size stays above maintenance all the way to the bound."
      : `You are liquidated if the odds reach ${cents(quote.liq)}.`;
    return [
      `If the odds go to ${fair}¢, you ${up >= 0n ? "make" : "lose"} ${dollars(up >= 0n ? up : -up)}.`,
      `If they go to ${cents(downPrice)}, you ${down >= 0n ? "make" : "lose"} ${dollars(down >= 0n ? down : -down)}.`,
      liq,
    ];
  }, [quote, long, fairPrice, fair, mark]);

  async function withWallet(run: (owner: import("@solana/web3.js").PublicKey) => Promise<void>) {
    if (!publicKey || !signTransaction) {
      setVisible(true);
      return;
    }
    try {
      await run(publicKey);
      await refresh();
    } catch (err) {
      setNote(plainError(err));
    } finally {
      setBusy("");
    }
  }

  async function onFaucet() {
    if (!mint) {
      setNote("The exchange is not on this cluster yet. Deploy and initialise a market first.");
      return;
    }
    setBusy("Getting devnet SOL and USDC. This usually takes a few seconds.");
    await withWallet(async (owner) => {
      const balance = await connection.getBalance(owner);
      if (balance < 50_000_000) {
        try {
          const sig = await connection.requestAirdrop(owner, 1_000_000_000);
          await connection.confirmTransaction(sig, "confirmed");
        } catch {
          setNote("The public SOL faucet is busy. Fund the wallet with solana airdrop, then press Get devnet USDC again.");
          return;
        }
      }
      const mintKey = new PublicKey(mint);
      const user = await connection.getAccountInfo(userPda(owner));
      const amount = 100_000_000n;
      const ixs = [];
      if (!user) ixs.push(initUserIx(owner));
      ixs.push(faucetIx(owner, mintKey, amount));
      ixs.push(depositIx(owner, mintKey, amount));
      const sig = await sendIxs(connection, owner, (tx) => signTransaction!(tx), ixs);
      setNote(`Devnet USDC is in the desk. ${dollars(amount)} deposited.`);
      setReceipt({
        sig,
        entry: "faucet",
        exit: "deposit",
        funding: "$0.00",
        net: dollars(amount),
      });
    });
  }

  async function onTrade() {
    if (!quote || example) {
      setNote(example ? "These numbers are an example. Live markets appear after deploy." : "Enter a risk amount.");
      return;
    }
    if (stale) {
      setNote(`Oracle is ${age} seconds old. New trades are blocked.`);
      return;
    }
    if (paused || settled) {
      setNote(settled ? "This market is settled. Only closes at the settlement price are left." : "Trading is paused.");
      return;
    }
    setBusy(long ? "Buying the odds." : "Selling the odds.");
    await withWallet(async (owner) => {
      const limit = long ? quote.fill + 20_000n : quote.fill > 20_000n ? quote.fill - 20_000n : 1_000n;
      const sig = await sendIxs(connection, owner, (tx) => signTransaction!(tx), [
        openIx(owner, market.pubkey, long ? 0 : 1, quote.size, limit),
      ]);
      setNote(`Filled ${contracts(quote.size)} contracts at ${cents(quote.fill)}.`);
      setReceipt({
        sig,
        entry: cents(quote.fill),
        exit: "open",
        funding: "$0.00",
        net: `margin ${dollars(quote.margin)}`,
      });
    });
  }

  async function onClose() {
    if (!position || position.size === 0n) return;
    setBusy("Closing the position.");
    await withWallet(async (owner) => {
      const size = position.size < 0n ? -position.size : position.size;
      const limit = position.size > 0n ? 0n : 1_000_000n;
      const exit = fillOf(mark, position.size < 0n, size);
      const trade = pnlOf(position.size, position.entry, exit);
      const funding = fundingPayment(position.size, premium);
      const fee = takerFee(size);
      const net = trade - funding - fee;
      const sig = await sendIxs(connection, owner, (tx) => signTransaction!(tx), [
        closeIx(owner, market.pubkey, size, limit),
      ]);
      const text = `Opened at ${cents(position.entry)}, closed at ${cents(exit)}. Funding ${dollars(funding)}. Net ${dollars(net)}.`;
      setNote(text);
      setReceipt({
        sig,
        entry: cents(position.entry),
        exit: cents(exit),
        funding: dollars(funding),
        net: dollars(net),
      });
    });
  }

  async function onFunding() {
    if (example) return;
    setBusy("Updating funding.");
    await withWallet(async (owner) => {
      const sig = await sendIxs(connection, owner, (tx) => signTransaction!(tx), [fundingIx(market.pubkey)]);
      setNote("Funding ticked. The line below is the new rate.");
      setReceipt({ sig, entry: cents(mark), exit: cents(market.index), funding: dollars(premium), net: "one interval" });
    });
  }

  const payee =
    premium > 0n
      ? market.oiShort === 0n
        ? "Longs pay the insurance fund"
        : "Longs pay shorts"
      : premium < 0n
        ? market.oiLong === 0n
          ? "Shorts pay the insurance fund"
          : "Shorts pay longs"
        : "Nobody pays";
  const hourPay = position && position.size !== 0n ? fundingPayment(position.size, premium) : premium;
  const nextIn = Math.max(0, Number(market.lastFundingTs) + 3600 - now);
  const disabled =
    example || stale || paused || settled || !quote
      ? example
        ? "Example only"
        : stale
          ? "Oracle is stale"
          : settled
            ? "Market settled"
            : paused
              ? "Trading is paused"
              : "Set a risk"
      : null;

  return (
    <div className="desk">
      <header className="top">
        <div className="brand">
          <Link href="/">Mark</Link>
          <span className="badge">Devnet</span>
        </div>
        <p className="lede">{note}</p>
        <div className="top-actions">
          <button type="button" className="ghost" onClick={() => void onFaucet()} disabled={!!busy}>
            Get devnet USDC
          </button>
          {connected ? (
            <button type="button" className="ghost" onClick={() => void disconnect()}>
              {publicKey?.toBase58().slice(0, 4)}…{publicKey?.toBase58().slice(-4)}
              {sol != null ? ` · ${(sol / 1e9).toFixed(2)} SOL` : ""}
            </button>
          ) : (
            <>
              <button type="button" className="ghost" onClick={() => setVisible(true)}>
                Connect
              </button>
              <button
                type="button"
                className="sand"
                onClick={() => {
                  select(DemoWalletName);
                  void connect().catch(() => setNote("The demo wallet is ready. Press it again if the first click only selected it."));
                }}
              >
                Try with a demo wallet
              </button>
            </>
          )}
        </div>
      </header>

      <div className="grid">
        <aside className="markets">
          {(markets.length ? markets : [EXAMPLE]).map((m, i) => (
            <button
              key={m.symbol + i}
              type="button"
              className={i === selected || (!live && i === 0) ? "market on" : "market"}
              onClick={() => setSelected(i)}
            >
              <span>{m.symbol || "MARKET"}</span>
              <strong className="num">{cents(m.mark)}</strong>
              <em>{example ? "example" : m.status === 1 ? "paused" : "funding 1h"}</em>
            </button>
          ))}
          <p className="fine">
            <Link href="/docs">What this is</Link>
          </p>
        </aside>

        <section className="stage">
          <h1>{market.question}</h1>
          <p className="resolution">{market.resolution}</p>
          {example && <p className="example-flag">Example. Not a live print.</p>}
          <Rail
            index={market.index}
            mark={mark}
            entry={position && position.size !== 0n ? position.entry : quote?.fill ?? null}
            liq={quote?.liq ?? null}
            long={long}
          />
          <Chart points={points} example={example || points.every((p) => p.example)} />
          {stale && <p className="warn">Oracle is {age} seconds old. New trades are blocked.</p>}
          {paused && <p className="warn">Trading is paused. You can still close.</p>}
        </section>

        <section className="ticket">
          <div className="sides">
            <button type="button" className={side === "buy" ? "buy on" : "buy"} onClick={() => setSide("buy")}>
              Buy
            </button>
            <button type="button" className={side === "sell" ? "sell on" : "sell"} onClick={() => setSide("sell")}>
              Sell
            </button>
          </div>
          <label>
            Risk, in dollars
            <input value={risk} onChange={(e) => setRisk(e.target.value)} inputMode="decimal" />
          </label>
          <label>
            I think this is worth, cents
            <input value={fair} onChange={(e) => setFair(e.target.value)} inputMode="decimal" />
          </label>
          <dl className="facts">
            <div>
              <dt>
                <Term id="Size" />
              </dt>
              <dd className="num">{quote ? contracts(quote.size) : "—"}</dd>
            </div>
            <div>
              <dt>Safe leverage</dt>
              <dd className="num">{quote ? `${quote.leverage.toFixed(2)}×` : "—"}</dd>
            </div>
            <div>
              <dt>
                <Term id="Margin" />
              </dt>
              <dd className="num">{quote ? dollars(quote.margin) : "—"}</dd>
            </div>
            <div>
              <dt>Free</dt>
              <dd className="num">{free == null ? "—" : dollars(free)}</dd>
            </div>
          </dl>
          <div className="sentences">
            {sentences.map((line) => (
              <p key={line}>{line}</p>
            ))}
          </div>
          <button type="button" className="sand sign" disabled={!!busy || !!disabled} onClick={() => void onTrade()}>
            {busy || disabled || (long ? "Buy the odds" : "Sell the odds")}
          </button>
          <p className="fine">Fee about {quote ? dollars(quote.fee) : "$0.00"}. Limit sits two cents past the quote.</p>
        </section>
      </div>

      <section className="bottom">
        <div className="tabs">
          {(["position", "funding", "tape"] as const).map((name) => (
            <button key={name} type="button" className={tab === name ? "on" : ""} onClick={() => setTab(name)}>
              {name === "position" ? "Position" : name === "funding" ? "Funding" : "Tape"}
            </button>
          ))}
        </div>
        {tab === "position" && (
          <div className="sheet">
            {position && position.size !== 0n ? (
              <p>
                {position.size > 0n ? "Long" : "Short"} {contracts(position.size < 0n ? -position.size : position.size)}{" "}
                from <Term id="Mark">{cents(position.entry)}</Term>. Margin {dollars(position.margin)}.{" "}
                <button type="button" className="ghost" onClick={() => void onClose()}>
                  Close
                </button>
              </p>
            ) : (
              <p className="ghost-row">Long 10 contracts from 51.0¢. Margin $2.55. This is what a position looks like.</p>
            )}
            {receipt && (
              <p className="receipt">
                {receipt.entry} → {receipt.exit}. Funding {receipt.funding}. Net {receipt.net}.{" "}
                <a href={solscan(receipt.sig)} target="_blank" rel="noreferrer">
                  Solscan
                </a>
              </p>
            )}
          </div>
        )}
        {tab === "funding" && (
          <div className="sheet">
            <p>
              {payee} {dollars(hourPay < 0n ? -hourPay : hourPay)} this hour because the perp is{" "}
              {Math.abs(basisCents).toFixed(1)}¢ {basisCents >= 0 ? "rich" : "cheap"}.
            </p>
            <p className="fine">
              <Term id="Funding" /> next tick in {Math.floor(nextIn / 60)}m {nextIn % 60}s.
              {!example && (
                <button type="button" className="ghost" onClick={() => void onFunding()}>
                  Update funding
                </button>
              )}
            </p>
          </div>
        )}
        {tab === "tape" && (
          <div className="sheet">
            <table>
              <thead>
                <tr>
                  <th>Source</th>
                  <th>Index</th>
                  <th>Spot</th>
                  <th>Age</th>
                  <th>Confidence</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>{market.oracleKind === 1 ? "Pyth model" : example ? "example" : "Published reference"}</td>
                  <td className="num">{cents(market.index)}</td>
                  <td className="num">{market.spot === 0n ? "—" : dollars(market.spot)}</td>
                  <td className="num">{age == null ? "—" : `${age}s`}</td>
                  <td className="num">{market.conf === 0n ? "point" : dollars(market.conf)}</td>
                  <td>{stale ? "fail" : example ? "example" : "ok"}</td>
                </tr>
              </tbody>
            </table>
            <p className="fine">{market.resolution}</p>
          </div>
        )}
      </section>
    </div>
  );
}

function Chart({ points, example }: { points: Point[]; example: boolean }) {
  if (points.length < 2) return null;
  const w = 100;
  const h = 36;
  const ys = points.flatMap((p) => [p.mark, p.index]);
  const lo = Math.min(...ys) - 0.02;
  const hi = Math.max(...ys) + 0.02;
  const path = (key: "mark" | "index") =>
    points
      .map((p, i) => {
        const x = (i / (points.length - 1)) * w;
        const y = h - ((p[key] - lo) / (hi - lo)) * h;
        return `${i === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(" ");
  return (
    <figure className="chart">
      <svg viewBox={`0 0 ${w} ${h}`} role="img" aria-label="Mark and index">
        <path d={path("index")} className="line-index" />
        <path d={path("mark")} className="line-mark" />
      </svg>
      <figcaption>
        <span>mark</span>
        <span>index</span>
        <span>probability</span>
        {example && <span>example</span>}
        <span>updated just now</span>
      </figcaption>
    </figure>
  );
}
