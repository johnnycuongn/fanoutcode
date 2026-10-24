import { describe, it, expect } from "vitest";
import { makeTask, makeProject, makeView } from "@/test/fixtures";
import { projectContainer, workspaceContainer } from "@/lib/stores/tasksStore";
import { buildCommands, type PaletteCtx } from "./commands";

const base: PaletteCtx = {
  kind: "project",
  tasks: [makeTask({ id: "task-b2", title: "Login" })],
  projects: [makeProject()],
  views: [makeView({ name: "Main" })],
  windows: [
    { window_id: "w-1", title: "Alpha", container: projectContainer("proj-a1"), project_ids: ["proj-a1"] },
    { window_id: "w-2", title: "WS", container: workspaceContainer("ws-d4"), project_ids: [] },
  ],
  currentWindowId: "w-1",
  focusedTask: null,
  mayCommit: false,
  focusTaskTerminal: () => {},
};
const labels = (ctx: PaletteCtx) => buildCommands(ctx).map((c) => c.label);

describe("buildCommands", () => {
  it("lists go-to targets, excluding the current window", () => {
    const l = labels(base);
    expect(l).toContain("Go to task: Login");
    expect(l).toContain("Go to view: Main");
    expect(l).toContain("Go to window: WS");
    expect(l).not.toContain("Go to window: Alpha");
    expect(l).toContain("New task");
    expect(l).toContain("New view");
    expect(l).toContain("Theme: Dark");
    expect(l).toContain("Add project to workspace…");
  });
  it("offers stage actions for the focused task by stage", () => {
    expect(labels({ ...base, focusedTask: makeTask({ stage: "Plan" }) })).toContain("Start build");
    expect(labels({ ...base, focusedTask: makeTask({ stage: "Test" }) })).toEqual(
      expect.arrayContaining(["Run tests", "Fix failing tests"]),
    );
    const commit = labels({ ...base, focusedTask: makeTask({ stage: "Commit" }) });
    expect(commit).toContain("Copy commit message");
    expect(commit).not.toContain("Commit");
    expect(labels({ ...base, focusedTask: makeTask({ stage: "Commit" }), mayCommit: true })).toContain("Commit");
  });
  it("adds Focus project entries only in workspaces", () => {
    expect(labels(base).some((l) => l.startsWith("Focus project"))).toBe(false);
    expect(labels({ ...base, kind: "workspace" })).toContain("Focus project: Alpha");
  });
});
