export type DocPage = {
  slug: string;
  title: string;
  summary: string[];
  sections: { heading: string; body: string[] }[];
};

export const DOCS: DocPage[] = [
  {
    slug: "",
    title: "What Mark is",
    summary: [
      "Mark is a perpetual on an event probability.",
      "You buy or sell the odds between zero and one, with margin that knows the price cannot leave that interval.",
      "Funding keeps the traded price honest to a published index.",
    ],
    sections: [
      {
        heading: "The gap",
        body: [
          "A YES share at 40 cents is fully prepaid. An asset perpetual will let you leverage bitcoin, not the view that bitcoin finishes the year above a strike.",
          "Mark is the instrument in between: isolated margin on the probability itself.",
        ],
      },
      {
        heading: "A number",
        body: [
          "One contract pays one dollar if the probability moves by 1.00, so a one-cent move is one cent per contract.",
          "A trader who risks ten dollars is shown the size, the margin the program will lock, and the liquidation price before signing.",
        ],
      },
    ],
  },
  {
    slug: "trade",
    title: "The one-screen trade",
    summary: [
      "Pick the market that is already selected.",
      "Leave the suggested ten-dollar risk, or type another.",
      "Read the three sentences, then sign.",
    ],
    sections: [
      {
        heading: "What the sentences are",
        body: [
          "The first uses the cents you typed as a fair value. The second steps ten cents against you, or stops at liquidation if that comes first.",
          "The third is the price where equity falls under maintenance. It is the same search the program uses.",
        ],
      },
      {
        heading: "After the fill",
        body: [
          "The position row shows side, size, entry, and locked margin. Close prints entry, exit, funding, and net, with a devnet Solscan link.",
        ],
      },
    ],
  },
  {
    slug: "pricing",
    title: "Mark, index, and funding",
    summary: [
      "The index is the reference probability.",
      "The mark is the index plus one cent for each contract of long-minus-short imbalance.",
      "Once an hour, longs pay when the mark is rich and shorts pay when it is cheap.",
    ],
    sections: [
      {
        heading: "Worked funding",
        body: [
          "Mark 52.0¢, index 50.0¢, basis two cents. The premium is two cents divided by ten, which is $0.002 per contract per hour.",
          "Sixty contracts pay $0.12. If no shorts are open, that payment sits in the insurance fund. If the book is balanced, the fund is flat and the longs have paid the shorts.",
        ],
      },
      {
        heading: "The fill",
        body: [
          "A market buy fills halfway from the old mark to the new mark. A round trip with no index move crosses the same price. The taker fee, 0.20% of the one-dollar contract notional, is the spread.",
        ],
      },
    ],
  },
  {
    slug: "margin",
    title: "Margin and the safe band",
    summary: [
      "Initial margin is the greater of 5% of the one-dollar notional and half the worst loss to zero or one.",
      "A long at 90 cents therefore posts more than a long at 50 cents.",
      "Maintenance is the same shape at half the rates. Liquidation is equity below maintenance.",
    ],
    sections: [
      {
        heading: "One contract",
        body: [
          "At 50¢, worst loss is $0.50 and half of that is $0.25. The 5% floor is $0.05, so margin is $0.25. Safe leverage on the one-dollar contract is 4×.",
          "At 90¢, worst loss is $0.90 and half of that is $0.45. Safe leverage falls to about 2.2×. The event got obvious, so the margin grew.",
        ],
      },
      {
        heading: "The band",
        body: [
          "For a one-contract long entered at 50¢ with $0.25 of margin and no funding owed, equity drops under maintenance at 33.3332¢. The rail draws that price.",
          "A short of a 10¢ probability has a large worst loss, because the odds can still run to one. A short of a 90¢ probability does not.",
        ],
      },
    ],
  },
  {
    slug: "oracles",
    title: "Oracles and the tape",
    summary: [
      "Price-threshold markets read a Pyth spot and turn it into a probability with a published model.",
      "The model is one half when spot equals the strike, and it reaches the bound when spot is a full strike away.",
      "Event markets use a written reference. Stale or wide prints are rejected.",
    ],
    sections: [
      {
        heading: "The model, labelled as a model",
        body: [
          "p = 0.50 + (spot − strike) / (2 × strike), then clamped to 0.001–0.999.",
          "It is not an implied market and the desk does not call it one. Confidence wider than 2.5% of price is refused.",
        ],
      },
      {
        heading: "Freshness",
        body: [
          "Opens, closes, liquidations, and funding all require a print younger than the market's max age, unless the market has already been settled.",
          "A failed transaction cannot store its own rejection. The desk says why the signature was refused, in the same sentence the program uses.",
        ],
      },
    ],
  },
  {
    slug: "liquidation",
    title: "Liquidation",
    summary: [
      "Anyone may liquidate a position whose equity is under maintenance.",
      "The close is at the mark. A penalty of 1% of contract notional is taken from the remaining equity.",
      "Twenty percent of that penalty pays the keeper. The rest goes to the insurance fund. Whatever is left returns to the trader.",
    ],
    sections: [
      {
        heading: "If the house cannot pay",
        body: [
          "Profits are paid by the insurance fund, which is the other side of the book.",
          "If the fund cannot cover a payout, the trader receives what the fund has, a shortfall is recorded, and the market pauses. The admin cannot seize collateral.",
        ],
      },
    ],
  },
  {
    slug: "architecture",
    title: "Architecture",
    summary: [
      "One Anchor program, mark_perp, holds the exchange, each market, each trader, and each position.",
      "Collateral is a six-decimal mint the exchange can faucet on devnet. Three token accounts — vault, insurance, fees — are program-derived.",
      "The desk, the keeper, and the tests all speak the same integer formulas.",
    ],
    sections: [
      {
        heading: "Path of a fill",
        body: [
          "The trader deposits into the vault. Open refreshes the index, settles funding, quotes the fill, locks margin, and moves open interest.",
          "A later crank writes the cumulative funding index. The payment moves when the position is touched. Close realises price PnL against the insurance fund.",
        ],
      },
    ],
  },
  {
    slug: "verify",
    title: "Verify it yourself",
    summary: [
      "From the repo root, anchor build && cargo test -p mark_perp --lib && cargo test --manifest-path sim/Cargo.toml.",
      "The library tests cover the formulas, including the 90¢ margin and the 33.3332¢ liquidation.",
      "The LiteSVM tests run the program: deposit, open, funding, close, stale oracle, pause, liquidation, shortfall, and settlement.",
    ],
    sections: [
      {
        heading: "A signature",
        body: [
          "node scripts/verify_tx.mjs <signature> reads a devnet transaction and prints the program logs.",
          "Match the program id in the log with the id in the README.",
        ],
      },
    ],
  },
  {
    slug: "glossary",
    title: "Glossary",
    summary: [
      "Every label on the desk has a ten-word definition.",
      "This page is the longer form.",
      "Nothing here is a different number from the program.",
    ],
    sections: [
      {
        heading: "Terms",
        body: [
          "Mark. The traded probability, equal to the index plus the inventory skew.",
          "Index. The reference probability, from a Pyth model or a written event rule.",
          "Funding. The hourly payment that charges the rich side of the basis.",
          "Margin. Collateral locked up front. It scales with the distance to a total loss.",
          "Size. Contract count. One contract, one dollar per full point of probability.",
          "Liquidation. A permissionless close when equity is under maintenance.",
          "Basis. Mark minus index.",
          "Safe band. The prices the open position survives. Drawn on the rail.",
        ],
      },
    ],
  },
];

export function findDoc(slug: string): DocPage {
  return DOCS.find((page) => page.slug === slug) ?? DOCS[0];
}
