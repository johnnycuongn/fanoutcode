// Whether the ⌘K palette is open, and its query.
import { createStore } from "@/lib/stores/createStore";

export const paletteStore = createStore<{ open: boolean; query: string }>({ open: false, query: "" });

export function openPalette(): void {
  paletteStore.set({ open: true, query: "" });
}

export function closePalette(): void {
  paletteStore.set({ open: false, query: "" });
}

export function togglePalette(): void {
  if (paletteStore.get().open) closePalette();
  else openPalette();
}
