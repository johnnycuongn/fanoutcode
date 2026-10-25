// The settings screens' types. Values are Plan 03's (Plan 04's for the terminal theme) Rust variant names (A16).
import type { ResolvedSettings } from "@/lib/bindings";

export type { TerminalPalette, TerminalTheme } from "@/lib/bindings";

/** Every key resolved for one scope (Plan 03's registry, A12; `terminal_theme` is Plan 04's row). */
export type Settings = ResolvedSettings;

/** A registry key; a screen that names any other key fails `npm run typecheck`. */
export type SettingKey = keyof Settings;
