// The module-store pattern: every feature store and shared store is built with this and nothing else (no state
// library). State lives in the module; components read it with useSyncExternalStore.
import { useSyncExternalStore } from "react";

export interface Store<T extends object> {
  get(): T;
  /** Shallow-merges a patch; a function replaces the state. No notification when the state is unchanged. */
  set(next: Partial<T> | ((state: T) => T)): void;
  subscribe(listener: () => void): () => void;
  /** The whole state; re-renders on every change. */
  use(): T;
  /** Re-renders only when `Object.is(select(prev), select(next))` is false. */
  useSelector<U>(select: (state: T) => U): U;
  /** Back to the initial state. */
  reset(): void;
}

export function createStore<T extends object>(initial: T): Store<T> {
  let state = initial;
  const listeners = new Set<() => void>();

  const get = () => state;
  const replace = (next: T) => {
    if (Object.is(state, next)) return;
    state = next;
    for (const listener of [...listeners]) listener();
  };
  const subscribe = (listener: () => void) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  };

  return {
    get,
    set: (next) => replace(typeof next === "function" ? next(state) : { ...state, ...next }),
    subscribe,
    use: () => useSyncExternalStore(subscribe, get, get),
    // `select` must return a stable value for unchanged input (no fresh objects or arrays), or every change re-renders.
    useSelector: (select) => useSyncExternalStore(subscribe, () => select(state), () => select(state)),
    reset: () => replace(initial),
  };
}
