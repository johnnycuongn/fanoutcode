// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
vi.mock("@/lib/theme", () => ({ applyTheme: vi.fn(), setThemePreference: vi.fn(), setAccentPreset: vi.fn() }));
import { mockBindings, commands } from "@/test/mockBindings";
import { settingsStore, appScope } from "../store";
import type { Settings } from "../types";
import { AppSettings } from "./AppSettings";

const app = {
  theme: "System",
  accent: "Blue",
  terminal_theme: { type: "FollowApp" },
  terminal_font_family: "JetBrains Mono",
  terminal_font_size: 13,
  terminal_ligatures: true,
  detail_level: "Simple",
  dock_position: "Bottom",
  model_provider: "ClaudeCli",
  team_view_arrangement: "HubGraph",
  team_view_placement: "DockGrows",
  team_canvas_layout: "AutoReflow",
  team_drill_in_lens: "ByFile",
  workspace_project_layer: "MultiRoot",
  workspace_board: "Swimlanes",
  cost_overview: false,
  scrollback_lines: 10000,
} as Settings;
describe("AppSettings", () => {
  beforeEach(() => {
    settingsStore.set({ byScope: { app }, loading: {} });
    commands.settingsSet.mockResolvedValue({ status: "ok", data: null });
  });
  it("renders every app-level field with its current value", () => {
    render(<AppSettings />);
    for (const label of [
      "Theme",
      "Accent",
      "Terminal theme",
      "Terminal font",
      "Terminal font size",
      "Ligatures",
      "Detail level",
      "Dock position",
      "Model provider",
      "Team view arrangement",
      "Team view placement",
      "Team canvas layout",
      "Team drill-in lens",
      "Workspace project layer",
      "Workspace board",
      "Cost overview",
      "Scrollback lines",
    ])
      expect(screen.getByText(label)).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "System" })).toBeChecked();
    expect(screen.getByRole("combobox", { name: "Model provider" })).toHaveTextContent("Claude Code");
    expect(screen.getByRole("combobox", { name: "Terminal theme" })).toHaveTextContent("Follow app");
    expect(screen.getByRole("switch", { name: "Cost overview" })).not.toBeChecked();
  });
  it("writes a changed value to the app scope", () => {
    render(<AppSettings />);
    fireEvent.click(screen.getByRole("radio", { name: "Right inspector" }));
    expect(commands.settingsSet).toHaveBeenCalledWith(appScope(), "dock_position", "Right");
    fireEvent.click(screen.getByRole("switch", { name: "Cost overview" }));
    expect(commands.settingsSet).toHaveBeenCalledWith(appScope(), "cost_overview", true);
    fireEvent.click(screen.getByRole("switch", { name: "Ligatures" }));
    expect(commands.settingsSet).toHaveBeenCalledWith(appScope(), "terminal_ligatures", false);
  });
});
