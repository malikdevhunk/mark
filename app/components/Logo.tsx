import Link from "next/link";

export function MarkGlyph({ size = 28 }: { size?: number }) {
  return (
    <svg
      className="mark-glyph"
      width={size}
      height={size}
      viewBox="0 0 32 32"
      aria-hidden="true"
    >
      <defs>
        <linearGradient id="mark-gilt" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#fff3c4" />
          <stop offset="45%" stopColor="#f3c56b" />
          <stop offset="100%" stopColor="#e07a32" />
        </linearGradient>
        <filter id="mark-glow" x="-40%" y="-40%" width="180%" height="180%">
          <feGaussianBlur stdDeviation="1.2" result="b" />
          <feMerge>
            <feMergeNode in="b" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>
      <circle cx="16" cy="16" r="13.2" fill="none" stroke="rgba(243,197,107,0.28)" strokeWidth="1.2" />
      <path
        d="M16 4.5 A11.5 11.5 0 1 1 6.8 23.4"
        fill="none"
        stroke="url(#mark-gilt)"
        strokeWidth="2.2"
        strokeLinecap="round"
        filter="url(#mark-glow)"
      />
      <circle cx="16" cy="16" r="2.1" fill="url(#mark-gilt)" />
      <line x1="16" y1="16" x2="16" y2="6.2" stroke="url(#mark-gilt)" strokeWidth="1.6" strokeLinecap="round" />
    </svg>
  );
}

export function Logo({ href = "/", size = 28 }: { href?: string; size?: number }) {
  const inner = (
    <>
      <MarkGlyph size={size} />
      <span>Mark</span>
    </>
  );
  if (!href) return <span className="brand-lockup">{inner}</span>;
  return (
    <Link href={href} className="brand-lockup" aria-label="Mark, home">
      {inner}
    </Link>
  );
}
