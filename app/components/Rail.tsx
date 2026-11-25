"use client";

import { cents } from "@/lib/math";

type Props = {
  index: bigint;
  mark: bigint;
  entry: bigint | null;
  liq: bigint | null;
  long: boolean;
};

function xOf(price: bigint): number {
  const n = Number(price) / 1_000_000;
  return 8 + Math.min(1, Math.max(0, n)) * 84;
}

type Mark = { x: number; label: string; kind: string; lane: number };

function place(items: { x: number; label: string; kind: string }[]): Mark[] {
  const min = 12;
  const sorted = [...items].sort((a, b) => a.x - b.x);
  const lane: number[] = [];
  for (let i = 0; i < sorted.length; i++) {
    const used = new Set<number>();
    for (let j = 0; j < i; j++) {
      if (Math.abs(sorted[i].x - sorted[j].x) < min) used.add(lane[j]);
    }
    let l = 0;
    while (used.has(l)) l += 1;
    lane[i] = l;
  }
  return sorted.map((m, i) => ({ ...m, lane: lane[i] }));
}

export function Rail({ index, mark, entry, liq, long }: Props) {
  const band =
    liq == null
      ? null
      : long
        ? { from: xOf(liq), to: xOf(1_000_000n) }
        : { from: xOf(0n), to: xOf(liq) };
  const near = liq != null && Math.abs(Number(mark - liq)) < 40_000;
  const raw = [
    { x: xOf(index), label: "Index", kind: "index" },
    { x: xOf(mark), label: "Mark", kind: "mark" },
  ];
  if (entry != null) raw.push({ x: xOf(entry), label: "Entry", kind: "entry" });
  if (liq != null) raw.push({ x: xOf(liq), label: "Liq", kind: "liq" });
  const markers = place(raw);
  const lanes = Math.max(1, ...markers.map((m) => m.lane + 1));
  const head = 5 + lanes * 5;
  const axis = head + 4;
  const height = axis + 12;

  return (
    <figure className="rail">
      <svg viewBox={`0 0 100 ${height}`} role="img" aria-label="Probability from 0 to 1">
        <defs>
          <linearGradient id="rail-band-fill" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%" stopColor="rgba(62,224,192,0.08)" />
            <stop offset="100%" stopColor="rgba(62,224,192,0.32)" />
          </linearGradient>
          <linearGradient id="rail-band-warm" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%" stopColor="rgba(240,195,106,0.12)" />
            <stop offset="100%" stopColor="rgba(240,195,106,0.4)" />
          </linearGradient>
          <filter id="rail-soft" x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur stdDeviation="0.6" result="b" />
            <feMerge>
              <feMergeNode in="b" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>
        <line x1="8" y1={axis} x2="92" y2={axis} className="rail-glow" />
        <line x1="8" y1={axis} x2="92" y2={axis} className="rail-axis" />
        {band && (
          <rect
            x={Math.min(band.from, band.to)}
            y={axis - 3}
            width={Math.abs(band.to - band.from)}
            height="6"
            className={near ? "rail-band warm" : "rail-band"}
          />
        )}
        {[0, 0.25, 0.5, 0.75, 1].map((t) => (
          <g key={t}>
            <line x1={8 + t * 84} y1={axis - 2} x2={8 + t * 84} y2={axis + 2} className="rail-tick" />
            <text x={8 + t * 84} y={axis + 8} textAnchor="middle" className="rail-num">
              {t.toFixed(2)}
            </text>
          </g>
        ))}
        {markers.map((m) => (
          <Marker key={m.kind} x={m.x} label={m.label} kind={m.kind} lane={m.lane} axis={axis} />
        ))}
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

function Marker({
  x,
  label,
  kind,
  lane,
  axis,
}: {
  x: number;
  label: string;
  kind: string;
  lane: number;
  axis: number;
}) {
  const y = 4.2 + lane * 5;
  return (
    <g className={`mk mk-${kind}`}>
      <line x1={x} y1={y + 1} x2={x} y2={axis} />
      <text x={x} y={y} textAnchor="middle">
        {label}
      </text>
      {kind === "mark" && <circle className="bead" cx={x} cy={axis} r="1.15" />}
    </g>
  );
}
