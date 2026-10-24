// The one reduced-motion hook every animated primitive reads (spec §11.3): springs become 150 ms eases, loops stop.
import { useSyncExternalStore } from "react";
import type { Transition } from "motion/react";

export const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

export const SPRING: Transition = { type: "spring", stiffness: 420, damping: 32 };
export const EASE_REDUCED: Transition = { duration: 0.15, ease: "easeOut" };
export const GLOW_MS = 400;
export const REFLOW_MS = 300;

export function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia(REDUCED_MOTION_QUERY).matches
    : false;
}

function subscribe(onChange: () => void): () => void {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return () => {};
  const query = window.matchMedia(REDUCED_MOTION_QUERY);
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
}

export function useReducedMotion(): boolean {
  return useSyncExternalStore(subscribe, prefersReducedMotion, () => false);
}

/** The reduced-motion ease when `reduced`, otherwise `full` (the spring by default). */
export function transitionFor(reduced: boolean, full: Transition = SPRING): Transition {
  return reduced ? EASE_REDUCED : full;
}
