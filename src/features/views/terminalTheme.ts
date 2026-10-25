// The terminal's palette: which bundled palette a `terminal_theme` setting picks, and the xterm theme read from the
// `--term-*` variables that palette sets in styles.css (so no colour value lives outside the theme file).
import type { ITheme } from "@xterm/xterm";
import type { TerminalPalette, TerminalTheme } from "@/lib/bindings";

export type PaletteName = "graphite" | "soft-white" | "one-dark" | "solarized-light" | "solarized-dark";

const PALETTES: Record<TerminalPalette, PaletteName> = {
  Graphite: "graphite",
  SoftWhite: "soft-white",
  OneDark: "one-dark",
  SolarizedLight: "solarized-light",
  SolarizedDark: "solarized-dark",
};

/** Follow app: Graphite in dark, Soft White in light. */
export function resolvePalette(setting: TerminalTheme, appTheme: "light" | "dark"): PaletteName {
  switch (setting.type) {
    case "FollowApp":
      return appTheme === "dark" ? "graphite" : "soft-white";
    case "AlwaysDark":
      return "graphite";
    case "AlwaysLight":
      return "soft-white";
    case "Named":
      return PALETTES[setting.palette];
  }
}

// xterm's ANSI slots in order, --term-ansi-0 … --term-ansi-15.
const ANSI = [
  "black",
  "red",
  "green",
  "yellow",
  "blue",
  "magenta",
  "cyan",
  "white",
  "brightBlack",
  "brightRed",
  "brightGreen",
  "brightYellow",
  "brightBlue",
  "brightMagenta",
  "brightCyan",
  "brightWhite",
] as const satisfies readonly (keyof ITheme)[];

/** The xterm theme from `el`'s `--term-*` variables; a missing one stays undefined so xterm keeps its default. */
export function readXtermTheme(el: HTMLElement): ITheme {
  const style = getComputedStyle(el);
  const read = (name: string): string | undefined => {
    const v = (style.getPropertyValue(name) || el.style.getPropertyValue(name)).trim();
    return v === "" ? undefined : v;
  };
  const theme: ITheme = {
    background: read("--term-bg"),
    foreground: read("--term-fg"),
    cursor: read("--term-cursor"),
    cursorAccent: read("--term-bg"),
    selectionBackground: read("--term-selection"),
  };
  ANSI.forEach((slot, i) => {
    theme[slot] = read(`--term-ansi-${i}`);
  });
  return theme;
}

/** Ligatures need the DOM renderer: the WebGL and canvas renderers draw one cell at a time. */
export function rendererFor(ligatures: boolean): "webgl" | "dom" {
  return ligatures ? "dom" : "webgl";
}
