// Resolved settings per scope, written optimistically through `settings_set`. Scopes follow the bindings' `Scope`
// (the app layer is implicit); `null` at a layer clears that layer's override (Plan 03).
import {
  commands,
  type Accent,
  type Container,
  type DetailLevel,
  type JsonValue,
  type ModelProvider,
  type ProjectId,
  type Scope,
  type TaskId,
  type ThemePreference,
  type WorkspaceId,
} from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { setAccentPreset, setThemePreference, type AccentPreset } from "@/lib/theme";
import type { SettingKey, Settings, TerminalTheme } from "./types";

export type SettingsState = { byScope: Record<string, Settings>; loading: Record<string, boolean> };

export const settingsStore = createStore<SettingsState>({ byScope: {}, loading: {} });

export function appScope(): Scope {
  return { workspace: null, project: null, task: null };
}
export function projectScope(id: ProjectId): Scope {
  return { workspace: null, project: id, task: null };
}
export function workspaceScope(id: WorkspaceId): Scope {
  return { workspace: id, project: null, task: null };
}
export function taskScope(id: TaskId): Scope {
  return { workspace: null, project: null, task: id };
}
export function scopeFor(container: Container): Scope {
  return container.type === "Project" ? projectScope(container.id) : workspaceScope(container.id);
}

export function scopeKey(scope: Scope): string {
  const parts = [
    scope.workspace && `workspace:${scope.workspace}`,
    scope.project && `project:${scope.project}`,
    scope.task && `task:${scope.task}`,
  ].filter(Boolean);
  return parts.length ? parts.join("/") : "app";
}

const isApp = (scope: Scope) => scopeKey(scope) === "app";

// The scopes loaded so far, so an app-level write can refresh the scopes that inherit it.
const known = new Map<string, Scope>();

function setLoading(key: string, on: boolean) {
  settingsStore.set((s) => ({ ...s, loading: { ...s.loading, [key]: on } }));
}

export async function loadSettings(scope: Scope): Promise<Settings | undefined> {
  const key = scopeKey(scope);
  known.set(key, scope);
  setLoading(key, true);
  try {
    const r = await commands.settingsGet(scope);
    if (r?.status === "ok") {
      const data = r.data as Settings;
      settingsStore.set((s) => ({ ...s, byScope: { ...s.byScope, [key]: data } }));
    }
  } finally {
    setLoading(key, false);
  }
  return settingsStore.get().byScope[key];
}

function applyAppearance(key: SettingKey, value: unknown) {
  if (key === "theme" && value) setThemePreference(value as ThemePreference);
  if (key === "accent" && typeof value === "string") setAccentPreset(value.toLowerCase() as AccentPreset);
}

export async function setSetting<K extends SettingKey>(scope: Scope, key: K, value: Settings[K] | null): Promise<void> {
  const sk = scopeKey(scope);
  known.set(sk, scope);
  const current = settingsStore.get().byScope[sk];
  if (current) settingsStore.set((s) => ({ ...s, byScope: { ...s.byScope, [sk]: { ...current, [key]: value } } }));
  if (isApp(scope)) applyAppearance(key, value);
  const r = await commands.settingsSet(scope, key, value as JsonValue);
  if (r?.status !== "ok") {
    const reloaded = await loadSettings(scope);
    if (isApp(scope) && reloaded) applyAppearance(key, reloaded[key]);
    return;
  }
  // Narrower scopes inherit this layer; refresh the ones on screen.
  for (const [k, s] of known) if (k !== sk) void loadSettings(s);
}

export function useSetting<K extends SettingKey>(scope: Scope, key: K): Settings[K] | undefined {
  const sk = scopeKey(scope);
  return settingsStore.useSelector((s) => s.byScope[sk]?.[key]);
}

/** Every key resolved at `scope` (the app scope by default); undefined until loaded. Plan 04's terminal reads it. */
export function useSettings(scope: Scope = appScope()): Settings | undefined {
  const sk = scopeKey(scope);
  return settingsStore.useSelector((s) => s.byScope[sk]);
}

export const THEMES: ThemePreference[] = ["System", "Light", "Dark"];
export const ACCENTS: Accent[] = ["Blue", "Violet", "Teal", "Orange"];
export const DETAIL_LEVELS: DetailLevel[] = ["Simple", "Medium", "Detailed"];
export const MODEL_PROVIDERS: Array<{ value: ModelProvider; label: string }> = [
  { value: "ClaudeCli", label: "Claude Code" },
  { value: "Api", label: "Anthropic API" },
  { value: "Local", label: "Local model" },
];
export const TERMINAL_THEMES: Array<{ id: string; label: string; value: TerminalTheme }> = [
  { id: "FollowApp", label: "Follow app", value: { type: "FollowApp" } },
  { id: "AlwaysDark", label: "Always dark", value: { type: "AlwaysDark" } },
  { id: "AlwaysLight", label: "Always light", value: { type: "AlwaysLight" } },
  { id: "Graphite", label: "Graphite", value: { type: "Named", palette: "Graphite" } },
  { id: "SoftWhite", label: "Soft White", value: { type: "Named", palette: "SoftWhite" } },
  { id: "OneDark", label: "One Dark", value: { type: "Named", palette: "OneDark" } },
  { id: "SolarizedLight", label: "Solarized Light", value: { type: "Named", palette: "SolarizedLight" } },
  { id: "SolarizedDark", label: "Solarized Dark", value: { type: "Named", palette: "SolarizedDark" } },
];

export function terminalThemeId(t: TerminalTheme): string {
  return t.type === "Named" ? t.palette : t.type;
}
