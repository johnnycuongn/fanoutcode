// The "agent working" indicator on the canvas's agent hubs (§11.3): a looping ring, static under reduced motion.
// A CSS ring stands in for the Rive file until one is authored (no .riv asset exists in the repo yet).
import type { ReactElement } from "react";
import { cx } from "@/components/ui";
import { useReducedMotion } from "@/lib/motion";

export function WorkingRing({ size = 28, className }: { size?: number; className?: string }): ReactElement {
  const reduced = useReducedMotion();
  return (
    <span
      role="img"
      aria-label="Working"
      data-ring={reduced ? "static" : "loop"}
      style={{ width: size, height: size }}
      className={cx(
        "inline-block shrink-0 rounded-full border-2 border-ok",
        !reduced && "animate-spin border-t-transparent [animation-duration:1.4s]",
        className,
      )}
    />
  );
}
