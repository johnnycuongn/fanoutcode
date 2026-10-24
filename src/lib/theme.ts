// The webview side of the `theme` and `accent` settings: resolves System from macOS, sets `data-theme` and
// `data-accent` on <html> before first paint, and mirrors the choice in localStorage so the first paint is right
// before any IPC returns. Also the swatch class helpers (task and project colours).

/** `domain::ThemePreference`'s serde form. */
export type ThemePreference = "System" | "Light" | "Dark";
/** The `data-theme` attribute values. */
export type ResolvedTheme = "light" | "dark";
export type AccentPreset = "blue" | "violet" | "teal" | "orange";

const THEME_KEY = "teamcode.theme";
const ACCENT_KEY = "teamcode.accent";
const THEMES: readonly ThemePreference[] = ["System", "Light", "Dark"];
const ACCENTS: readonly AccentPreset[] = ["blue", "violet", "teal", "orange"];
const DARK_QUERY = "(prefers-color-scheme: dark)";

let preference: ThemePreference = "System";

export function resolveTheme(pref: ThemePreference, prefersDark: boolean): ResolvedTheme {
  if (pref === "System") return prefersDark ? "dark" : "light";
  return pref === "Dark" ? "dark" : "light";
}

export function systemPrefersDark(): boolean {
  try {
    return typeof matchMedia === "function" && matchMedia(DARK_QUERY).matches;
  } catch {
    return false;
  }
}

export function applyTheme(pref: ThemePreference, prefersDark = systemPrefersDark()): ResolvedTheme {
  const resolved = resolveTheme(pref, prefersDark);
  document.documentElement.dataset.theme = resolved;
  return resolved;
}

function readStored<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  try {
    const value = localStorage.getItem(key);
    return allowed.includes(value as T) ? (value as T) : fallback;
  } catch {
    return fallback; // private mode or blocked storage: never block first paint
  }
}

function writeStored(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // The setting in SQLite stays the source of truth; the mirror is only for first paint.
  }
}

export function readStoredTheme(): ThemePreference {
  return readStored(THEME_KEY, THEMES, "System");
}

export function storeTheme(pref: ThemePreference): void {
  writeStored(THEME_KEY, pref);
}

/** The one call the settings screen makes when the theme changes. */
export function setThemePreference(pref: ThemePreference): void {
  preference = pref;
  storeTheme(pref);
  applyTheme(pref);
}

export function applyAccent(preset: AccentPreset): void {
  if (preset === "blue") delete document.documentElement.dataset.accent;
  else document.documentElement.dataset.accent = preset;
}

export function readStoredAccent(): AccentPreset {
  return readStored(ACCENT_KEY, ACCENTS, "blue");
}

export function setAccentPreset(preset: AccentPreset): void {
  writeStored(ACCENT_KEY, preset);
  applyAccent(preset);
}

/** Applies the stored theme and accent, follows macOS while the preference is System; returns the unsubscribe. */
export function startThemeSync(): () => void {
  preference = readStoredTheme();
  applyTheme(preference);
  applyAccent(readStoredAccent());
  if (typeof matchMedia !== "function") return () => {};
  const query = matchMedia(DARK_QUERY);
  const onChange = (e: { matches: boolean }) => {
    if (preference === "System") applyTheme("System", e.matches);
  };
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
}

/** Same order as `domain::SWATCHES`. */
export const SWATCHES = ["rose", "orange", "yellow", "lime", "emerald", "cyan", "indigo", "violet", "fuchsia", "slate"] as const;
export type Swatch = (typeof SWATCHES)[number];
export type SwatchPart = "bg" | "text" | "edge";

// Written out literally: Tailwind v4 only generates classes it finds in source.
const SWATCH_CLASSES: Record<Swatch, Record<SwatchPart, string>> = {
  rose: { bg: "bg-swatch-rose", text: "text-swatch-rose", edge: "border-swatch-rose" },
  orange: { bg: "bg-swatch-orange", text: "text-swatch-orange", edge: "border-swatch-orange" },
  yellow: { bg: "bg-swatch-yellow", text: "text-swatch-yellow", edge: "border-swatch-yellow" },
  lime: { bg: "bg-swatch-lime", text: "text-swatch-lime", edge: "border-swatch-lime" },
  emerald: { bg: "bg-swatch-emerald", text: "text-swatch-emerald", edge: "border-swatch-emerald" },
  cyan: { bg: "bg-swatch-cyan", text: "text-swatch-cyan", edge: "border-swatch-cyan" },
  indigo: { bg: "bg-swatch-indigo", text: "text-swatch-indigo", edge: "border-swatch-indigo" },
  violet: { bg: "bg-swatch-violet", text: "text-swatch-violet", edge: "border-swatch-violet" },
  fuchsia: { bg: "bg-swatch-fuchsia", text: "text-swatch-fuchsia", edge: "border-swatch-fuchsia" },
  slate: { bg: "bg-swatch-slate", text: "text-swatch-slate", edge: "border-swatch-slate" },
};

/** `bg-`, `text-` or `border-swatch-<name>` (`edge` is the border colour only); unknown names fall back to slate. */
export function swatchClass(swatch: string, part: SwatchPart): string {
  const classes = (SWATCH_CLASSES as Record<string, Record<SwatchPart, string>>)[swatch] ?? SWATCH_CLASSES.slate;
  return classes[part];
}
