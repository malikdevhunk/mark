"use client";

import Link from "next/link";
import type { ReactNode } from "react";
import { GLOSSARY, type TermId } from "@/lib/glossary";

export function Term({ id, children }: { id: TermId; children?: ReactNode }) {
  const term = GLOSSARY[id];
  return (
    <span className="term">
      <span className="term-label">{children ?? id}</span>
      <span className="term-pop" role="tooltip">
        <span>{term.short}</span>
        <Link href={term.href}>More</Link>
      </span>
    </span>
  );
}
