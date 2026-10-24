// Turns the Promise<Unlisten> that `ipc.on` returns into a function that is safe to call at any time: before the
// listener resolves it cancels, after it resolves it unlistens at once.
import type { Unlisten } from "./ipc";

export function track(pending: Promise<Unlisten>): Unlisten {
  let off: Unlisten | null = null;
  let cancelled = false;
  void pending.then((un) => {
    if (cancelled) un();
    else off = un;
  });
  return () => {
    cancelled = true;
    off?.();
    off = null;
  };
}

/** Combines several unlisten functions into one. */
export function all(...offs: Unlisten[]): Unlisten {
  return () => offs.forEach((off) => off());
}
