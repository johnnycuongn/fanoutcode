// The palette feature's public surface.
export { paletteStore, openPalette, closePalette, togglePalette } from "./store";
export { buildCommands } from "./commands";
export type { PaletteCommand, PaletteCtx, PaletteGroup } from "./commands";
export { CommandPalette } from "./components/CommandPalette";
