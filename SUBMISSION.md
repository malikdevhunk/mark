# Submission notes — Mark

**Title:** Mark

**One-liner:** Perpetual markets on event odds.

**Description:** Mark lets a trader long or short an event probability between zero and one. Margin is sized from the worst loss to the bound, so a 90 cent long posts more than a 50 cent long. Funding, paid once an hour, pulls the traded mark back to a published index. The desk says, before the signature, what the trader makes, what they lose, and where they are liquidated. Built and developed by Saddam Malik.

**Tech:** Solana devnet, Anchor 1.2 (Rust), Pyth price accounts for the model index, Next.js, wallet-adapter (Phantom, Solflare, Backpack and MetaMask via the Wallet Standard, plus a devnet demo wallet).

**Track:** Main track only. The pm-AMM bounty is an investment, not a grant, and is not entered.

**Team:** Saddam Malik, solo. India.

## Judging

| What the judges have asked for | Where it is |
|---|---|
| A real user and a real problem | Someone who has a view on an event, and can today only buy a prepaid share or leverage the asset instead of the odds. |
| A working loop | Deposit, open, funding crank, close, liquidate. `cargo test -p mark_perp --lib` and `cargo test --manifest-path sim/Cargo.toml`. |
| Why Solana | Funding and liquidations are cheap cranks. Pyth already posts here. The desk makes many small updates. |
| Execution | One screen, the safe band, the three-sentence ticket, the oracle tape, the docs. |

## Pitch video — target 2:25, hard cap 3:00

No music under the voice. No avatar. No sting longer than a second.

1. **0:00–0:20.** "If you think ‘BTC above $150,000 this year’ is 40 cents and should be 55, today you buy a share one-for-one, or you sit out. Perp venues will let you leverage Bitcoin. They will not let you leverage that view."
2. **0:20–0:40.** "Mark is a perpetual on the probability itself. You long or short the odds, with margin that understands the price can only run to zero or one, and with funding that keeps the perp honest to its index. Built and developed by Saddam Malik."
3. **0:40–1:50.** The desk, live. Land on devnet. Faucet. The ticket is already filled. Read the three sentences while they are on screen. Sign. Point at the safe band: entry, mark, liquidation. Open funding and say who pays whom. Open the tape. Close, and read the receipt. Solscan sits in the corner.
4. **1:50–2:10.** "Prepaid YES/NO markets and asset perps already exist. A funded odds perpetual with a visible safe band does not."
5. **2:10–2:25.** "Funding and liquidations are cranks that should run all day. Pyth is already on Solana. Settlement is a fraction of a cent."
6. **2:25–2:30.** "It is live on devnet. The repo, the docs, and a technical walkthrough are in the submission."

## Technical video — target 4:00, hard cap 5:00

1. **0:00–0:30.** The account diagram, then `open_or_increase` on screen.
2. **0:30–1:30.** Bounded margin. One contract at 90 cents posts $0.45. One contract at 50 cents posts $0.25. Show the unit test that locks those numbers.
3. **1:30–2:20.** A stale oracle reverts. A fresh print is accepted. The tape shows age and confidence.
4. **2:20–3:00.** Funding sign when the mark is rich, then a liquidation: keeper bounty, leftover to the trader.
5. **3:00–3:40.** `cargo test` on the LiteSVM suite, then `node scripts/verify_tx.mjs` on a devnet signature.
6. **3:40–4:00.** Where to look: `programs/mark_perp`, `sim`, `app`, `docs`. Unaudited devnet. Not an offer of a live exchange.

## Deck, nine slides

1. Mark — perpetual markets on event odds. Saddam Malik. saddammalik.com.
2. Asset perps, prepaid YES/NO, and the missing piece: an odds perpetual.
3. Index, mark from skew, funding, isolated position.
4. Bounded margin. 90 cents versus 50 cents, and the safe band.
5. The ticket. Screenshot of the three sentences.
6. The tape. Oracle, age, confidence, the settlement sentence.
7. Why Solana. Cranks, Pyth, cost.
8. What is live. Program id, markets, app, `cargo test`, docs. Only numbers that exist.
9. Not in this build: a fixed-loss ticket and auto-reduce. Labelled as not built.

## Live path for a judge

1. Open the app. The badge says Devnet.
2. Connect Phantom, Solflare, Backpack, or MetaMask's Solana account. Or press "Try with a demo wallet".
3. Press "Get devnet USDC". The ticket shows the balance when the deposit lands.
4. Read the question and the three sentences. The safe band matches the liquidation sentence.
5. Sign. Open the Solscan link. The program id matches this file and the README.
6. Open Funding. If an hour has passed, press "Update funding".
7. Open Tape. The last print has a time and a confidence.
8. Close. Read the receipt.
9. Stop. The docs sit at `/docs`.
