import { lazy, Suspense, useEffect, useState, type ReactElement } from "react";
import { useReducedMotion } from "@/lib/motion";
import doneArt from "../assets/done-moment.lottie?url";

const DoneLottie = lazy(() => import("./DoneLottie"));

const MAX_MS = 1_000;

/**
 * A one-shot flourish in the Done column header when a card lands there. Each new `playKey` plays it once; it goes
 * away when the animation completes or after a second. Under reduced motion it does not play at all.
 */
export function DoneMoment({ playKey }: { playKey: number | null }): ReactElement | null {
  const reduced = useReducedMotion();
  const [seen, setSeen] = useState<number | null>(null);
  const [playing, setPlaying] = useState<number | null>(null);
  if (playKey !== null && playKey !== seen) {
    setSeen(playKey);
    setPlaying(playKey);
  }
  useEffect(() => {
    if (playing === null) return;
    const timer = setTimeout(() => setPlaying(null), MAX_MS);
    return () => clearTimeout(timer);
  }, [playing]);

  if (playing === null || reduced) return null;
  return (
    <span data-testid="done-moment" className="pointer-events-none absolute -top-2 right-1 size-12">
      <Suspense fallback={null}>
        <DoneLottie key={playing} src={doneArt} size={48} onComplete={() => setPlaying(null)} />
      </Suspense>
    </span>
  );
}
