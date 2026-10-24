// Elapsed time in the current state, without a per-second re-render of every card: one shared one-second ticker, and
// each `useElapsed` snapshot is the formatted *string*, so React re-renders a leaf only when its text changes.
import { useSyncExternalStore } from "react";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** `0m`, `Nm`, `Nh MMm`, `Nd Nh`; a negative or NaN duration reads `0m`. */
export function formatElapsed(ms: number): string {
  if (!Number.isFinite(ms) || ms < MINUTE) return "0m";
  if (ms < HOUR) return `${Math.floor(ms / MINUTE)}m`;
  if (ms < DAY) {
    const minutes = Math.floor((ms % HOUR) / MINUTE);
    return `${Math.floor(ms / HOUR)}h ${String(minutes).padStart(2, "0")}m`;
  }
  return `${Math.floor(ms / DAY)}d ${Math.floor((ms % DAY) / HOUR)}h`;
}

const listeners = new Set<() => void>();
let timer: ReturnType<typeof setInterval> | undefined;

export const tickStore = {
  subscribe(listener: () => void): () => void {
    listeners.add(listener);
    if (listeners.size === 1) timer = setInterval(() => listeners.forEach((fn) => fn()), 1000);
    return () => {
      listeners.delete(listener);
      if (listeners.size === 0) {
        clearInterval(timer);
        timer = undefined;
      }
    };
  },
  now: (): number => Date.now(),
};

export function useElapsed(sinceIso: string): string {
  const snapshot = () => formatElapsed(tickStore.now() - Date.parse(sinceIso));
  return useSyncExternalStore(tickStore.subscribe, snapshot, snapshot);
}
