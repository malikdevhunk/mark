"use client";

type Props = { mark: bigint; index: bigint; size?: number };

function polar(cx: number, cy: number, r: number, t: number) {
  const a = Math.PI * (1 + t);
  return [cx + r * Math.cos(a), cy + r * Math.sin(a)] as const;
}

export function OddsDial({ mark, index, size = 112 }: Props) {
  const m = Math.min(1, Math.max(0, Number(mark) / 1_000_000));
  const i = Math.min(1, Math.max(0, Number(index) / 1_000_000));
  const cx = 50;
  const cy = 52;
  const r = 36;
  const [mx, my] = polar(cx, cy, r, m);
  const [ix, iy] = polar(cx, cy, r, i);
  const large = m > 0.5 ? 1 : 0;
  const arc = `M 14 52 A ${r} ${r} 0 ${large} 1 ${mx} ${my}`;
  return (
    <svg className="odds-dial" width={size} height={size * 0.72} viewBox="0 0 100 70" aria-hidden="true">
      <defs>
        <linearGradient id="dial-gilt" x1="0" y1="1" x2="1" y2="0">
          <stop offset="0%" stopColor="#e07a32" />
          <stop offset="55%" stopColor="#f3c56b" />
          <stop offset="100%" stopColor="#fff3c4" />
        </linearGradient>
        <filter id="dial-glow" x="-20%" y="-20%" width="140%" height="140%">
          <feGaussianBlur stdDeviation="1.6" result="b" />
          <feMerge>
            <feMergeNode in="b" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>
      <path d="M 14 52 A 36 36 0 1 1 86 52" fill="none" stroke="rgba(243,197,107,0.16)" strokeWidth="6" strokeLinecap="round" />
      <path d={arc} fill="none" stroke="url(#dial-gilt)" strokeWidth="6" strokeLinecap="round" filter="url(#dial-glow)" />
      <line x1={ix} y1={iy} x2={cx + (ix - cx) * 0.82} y2={cy + (iy - cy) * 0.82} stroke="#9aa0b4" strokeWidth="1.6" strokeLinecap="round" />
      <line x1={cx} y1={cy} x2={mx} y2={my} stroke="url(#dial-gilt)" strokeWidth="1.4" strokeLinecap="round" />
      <circle cx={mx} cy={my} r="3.1" fill="#fff3c4" filter="url(#dial-glow)" />
      <text x="14" y="66" textAnchor="middle" className="dial-cap">0</text>
      <text x="86" y="66" textAnchor="middle" className="dial-cap">1</text>
      <text x="50" y="66" textAnchor="middle" className="dial-cap">odds</text>
    </svg>
  );
}
