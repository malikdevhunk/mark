export type TermId =
  | "Mark"
  | "Index"
  | "Funding"
  | "Margin"
  | "Size"
  | "Liquidation"
  | "Basis"
  | "Safe band";

export const GLOSSARY: Record<
  TermId,
  { short: string; more: string; href: string }
> = {
  Mark: {
    short: "The price you trade, nudged by who is long or short.",
    more: "Mark starts at the index and moves one cent per contract of imbalance. Buys lift it. Sells hit it.",
    href: "/docs/pricing",
  },
  Index: {
    short: "The reference probability the perp is trying to track.",
    more: "Price-threshold markets derive it from a Pyth spot with a published model. Event markets use a written reference.",
    href: "/docs/oracles",
  },
  Funding: {
    short: "The hourly payment that pulls mark back toward index.",
    more: "When mark is rich, longs pay. When it is cheap, shorts pay. The insurance fund holds the imbalance.",
    href: "/docs/pricing",
  },
  Margin: {
    short: "Collateral locked against the worst move to 0 or 1.",
    more: "Initial margin is the greater of a small floor and half the worst-case loss. Obvious events demand more.",
    href: "/docs/margin",
  },
  Size: {
    short: "Contracts. One contract pays $1 if probability moves by 1.00.",
    more: "A one-cent move is one cent of profit or loss per contract, before fees and funding.",
    href: "/docs/trade",
  },
  Liquidation: {
    short: "A keeper closes you when equity falls under maintenance.",
    more: "The close is at the mark. A 1% penalty is split between the keeper and the insurance fund. The rest comes back.",
    href: "/docs/liquidation",
  },
  Basis: {
    short: "Mark minus index, in cents. The funding driver.",
    more: "A two-cent rich perp charges longs $0.002 per contract per hour at the default rate.",
    href: "/docs/pricing",
  },
  "Safe band": {
    short: "The prices this position can survive before liquidation.",
    more: "The shaded stretch on the rail is the same band the program uses. It is not a sketch.",
    href: "/docs/margin",
  },
};
