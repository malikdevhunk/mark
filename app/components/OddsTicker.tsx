"use client";

import { animate, motion, useMotionValue, useTransform } from "framer-motion";
import { useEffect } from "react";
import { cents } from "@/lib/math";

export function OddsTicker({ micro, className }: { micro: bigint; className?: string }) {
  const value = useMotionValue(Number(micro) / 10_000);
  const rounded = useTransform(value, (v) => `${v.toFixed(1)}¢`);
  useEffect(() => {
    const controls = animate(value, Number(micro) / 10_000, { duration: 0.55, ease: "easeOut" });
    return () => controls.stop();
  }, [micro, value]);
  return (
    <motion.span className={className} aria-label={cents(micro)}>
      {rounded}
    </motion.span>
  );
}
