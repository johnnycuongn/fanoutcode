// Test helpers shared by the primitives' tests.
import { render, type RenderResult } from "@testing-library/react";
import userEvent, { type UserEvent } from "@testing-library/user-event";
import { createElement, type ReactElement } from "react";
import { UiProvider } from "@/components/ui/UiProvider";
import { REDUCED_MOTION_QUERY } from "@/lib/motion";

type Listener = (event: { matches: boolean }) => void;

/** Installs a `matchMedia` stub; only the reduced-motion query can be true. `set` flips it and notifies listeners. */
export function mockMatchMedia(reduced: boolean): { set(reduced: boolean): void } {
  let current = reduced;
  const listeners = new Map<string, Set<Listener>>();
  const matchMedia = (query: string) => {
    const set = listeners.get(query) ?? new Set<Listener>();
    listeners.set(query, set);
    return {
      get matches() {
        return query === REDUCED_MOTION_QUERY ? current : false;
      },
      media: query,
      onchange: null,
      addEventListener: (_type: string, l: Listener) => set.add(l),
      removeEventListener: (_type: string, l: Listener) => set.delete(l),
      addListener: (l: Listener) => set.add(l),
      removeListener: (l: Listener) => set.delete(l),
      dispatchEvent: () => false,
    };
  };
  Object.defineProperty(window, "matchMedia", { value: matchMedia, configurable: true, writable: true });
  return {
    set(next) {
      current = next;
      for (const l of listeners.get(REDUCED_MOTION_QUERY) ?? []) l({ matches: next });
    },
  };
}

export function setupUser(): UserEvent {
  return userEvent.setup({ pointerEventsCheck: 0 });
}

/** `render` inside the UiProvider (tooltips need it). */
export function renderUi(ui: ReactElement): RenderResult {
  return render(createElement(UiProvider, null, ui));
}
