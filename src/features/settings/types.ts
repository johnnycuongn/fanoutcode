// The settings screens' types. Values are Plan 03's (Plan 04's for the terminal theme) Rust variant names (A16).
import type { ResolvedSettings } from "@/lib/bindings";

// STUB(plan 04): Plan 04's `sessions::theme::TerminalTheme` / `TerminalPalette` and its `terminal_theme` key, not in
// the bindings yet. When Plan 04 merges, import both types from "@/lib/bindings" and make `Settings` an alias.
export type TerminalPalette = "Graphite" | "SoftWhite" | "OneDark" | "SolarizedLight" | "SolarizedDark";
export type TerminalTheme =
  | { type: "FollowApp" }
  | { type: "AlwaysDark" }
  | { type: "AlwaysLight" }
  | { type: "Named"; palette: TerminalPalette };

/** Every key resolved for one scope (Plan 03's registry, A12). */
export type Settings = ResolvedSettings & { terminal_theme: TerminalTheme };

/** A registry key; a screen that names any other key fails `npm run typecheck`. */
export type SettingKey = keyof Settings;
