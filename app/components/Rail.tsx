"use client";

import { cents } from "@/lib/math";

type Props = {
  index: bigint;
  mark: bigint;
  entry: bigint | null;
  liq: bigint | null;
  long: boolean;
};

function x(price: bigint): number {
  const n = Number(price) / 1_000_000;
  return 8 + Math.min(1, Math.max(0, n)) * 84;
}

export function Rail({ index, mark, entry, liq, long }: Props) {
  const band =
    liq == null
      ? null
      : long
        ? { from: x(liq), to: x(1_000_000n) }
        : { from: x(0n), to: x(liq) };
  const near =
    liq != null && Math.abs(Number(mark - liq)) < 40_000;
  return (
    <figure className="rail">
      <svg viewBox="0 0 100 28" role="img" aria-label="Probability from 0 to 1">
        <line x1="8" y1="16" x2="92" y2="16" className="rail-axis" />
        {band && (
          <rect
            x={Math.min(band.from, band.to)}
            y="13"
            width={Math.abs(band.to - band.from)}
            height="6"
            className={near ? "rail-band warm" : "rail-band"}
          />
        )}
        {[0, 0.25, 0.5, 0.75, 1].map((t) => (
          <g key={t}>
            <line x1={8 + t * 84} y1="14" x2={8 + t * 84} y2="18" className="rail-tick" />
            <text x={8 + t * 84} y="26" textAnchor="middle" className="rail-num">
              {t.toFixed(2)}
            </text>
          </g>
        ))}
        <Marker x={x(index)} label="Index" kind="index" />
        <Marker x={x(mark)} label="Mark" kind="mark" />
        {entry != null && <Marker x={x(entry)} label="Entry" kind="entry" />}
        {liq != null && <Marker x={x(liq)} label="Liq" kind="liq" />}
      </svg>
      <figcaption>
        <span>0</span>
        <span>probability</span>
        <span>1</span>
        <span className="rail-read">
          mark {cents(mark)} · index {cents(index)}
          {liq != null ? ` · liquidation ${cents(liq)}` : ""}
        </span>
      </figcaption>
    </figure>
  );
}

function Marker({ x, label, kind }: { x: number; label: string; kind: string }) {
  return (
    <g className={`mk mk-${kind}`}>
      <line x1={x} y1="6" x2={x} y2="16" />
      <text x={x} y="5" textAnchor="middle">
        {label}
      </text>
    </g>
  );
}
