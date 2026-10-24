import { describe, it, expect, vi, beforeEach } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
vi.mock("@/lib/theme", () => ({ applyTheme: vi.fn(), setThemePreference: vi.fn(), setAccentPreset: vi.fn() }));
import { mockBindings, commands } from "@/test/mockBindings";
import { setThemePreference, setAccentPreset } from "@/lib/theme";
import { settingsStore, loadSettings, setSetting, appScope, projectScope, scopeKey } from "./store";
import type { Settings } from "./types";

const resolved = {
  theme: "System",
  accent: "Blue",
  detail_level: "Simple",
  dock_position: "Bottom",
  cost_overview: false,
  first_run_complete: true,
} as Settings;

describe("settingsStore", () => {
  beforeEach(() => {
    settingsStore.set({ byScope: {}, loading: {} });
    vi.clearAllMocks();
  });
  it("keys scopes distinctly", () => {
    expect(scopeKey(appScope())).toBe("app");
    expect(scopeKey(projectScope("proj-a1"))).toBe("project:proj-a1");
  });
  it("loads and caches a scope", async () => {
    commands.settingsGet.mockResolvedValue({ status: "ok", data: resolved });
    await loadSettings(projectScope("proj-a1"));
    expect(settingsStore.get().byScope["project:proj-a1"].detail_level).toBe("Simple");
  });
  it("sets optimistically and applies the theme for app-scope theme keys", async () => {
    settingsStore.set({ byScope: { app: resolved } });
    commands.settingsSet.mockResolvedValue({ status: "ok", data: null });
    await setSetting(appScope(), "theme", "Dark");
    expect(commands.settingsSet).toHaveBeenCalledWith(appScope(), "theme", "Dark");
    expect(settingsStore.get().byScope.app.theme).toBe("Dark");
    expect(setThemePreference).toHaveBeenCalledWith("Dark");
    await setSetting(appScope(), "accent", "Teal");
    expect(setAccentPreset).toHaveBeenCalledWith("teal");
  });
  it("reloads the scope when the write fails", async () => {
    settingsStore.set({ byScope: { app: resolved } });
    commands.settingsSet.mockResolvedValue({ status: "error", error: "Db(locked)" });
    commands.settingsGet.mockResolvedValue({ status: "ok", data: { ...resolved, dock_position: "Bottom" } });
    await setSetting(appScope(), "dock_position", "Right");
    expect(settingsStore.get().byScope.app.dock_position).toBe("Bottom");
  });
});
