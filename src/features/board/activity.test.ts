import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("./ipc");
import {
  activityStore,
  applyChanges,
  emptyActivity,
  loadActivity,
  reduceActivity,
  startActivityEvents,
} from "./activity";
import type { ChangedFile, HookEvent } from "@/lib/bindings";
import { makeEvent, projectContainer } from "./testing/fixtures";
import { mockIpc } from "./testing/mockIpc";

const edit = (file_path: string): HookEvent => ({
  type: "PostToolUse",
  tool: "Edit",
  input: { file_path },
  response: {},
});

describe("reduceActivity", () => {
  it('Edit sets the current file and the last line "Editing <basename>"', () => {
    const a = reduceActivity(emptyActivity, edit("/repo/src/store.ts"));
    expect(a.currentFile).toEqual({ path: "/repo/src/store.ts", added: 0, removed: 0 });
    expect(a.lastLine).toBe("Editing store.ts");
  });
  it.each(["Write", "MultiEdit", "NotebookEdit"])("%s also sets the current file", (tool) => {
    const key = tool === "NotebookEdit" ? "notebook_path" : "file_path";
    const a = reduceActivity(emptyActivity, { type: "PostToolUse", tool, input: { [key]: "/r/a.ts" }, response: {} });
    expect(a.currentFile?.path).toBe("/r/a.ts");
  });
  it("editing the same file again keeps its +/- counts and a new file resets them", () => {
    const counted = { ...emptyActivity, currentFile: { path: "/r/a.ts", added: 4, removed: 1 } };
    expect(reduceActivity(counted, edit("/r/a.ts")).currentFile).toEqual({ path: "/r/a.ts", added: 4, removed: 1 });
    expect(reduceActivity(counted, edit("/r/b.ts")).currentFile).toEqual({ path: "/r/b.ts", added: 0, removed: 0 });
  });
  it("Bash sets the last line to the command trimmed to 80 characters", () => {
    const a = reduceActivity(emptyActivity, {
      type: "PostToolUse",
      tool: "Bash",
      input: { command: "npm test " + "x".repeat(100) },
      response: {},
    });
    expect(a.lastLine!.startsWith("$ npm test")).toBe(true);
    expect(a.lastLine).toHaveLength(82);
  });
  it("Stop sets the last line to the first non-sentinel line and leaves it when only the sentinel is present", () => {
    const a = reduceActivity(emptyActivity, {
      type: "Stop",
      last_assistant_message: "All steps done.\nTEAMCODE: build done",
      background_tasks: 0,
    });
    expect(a.lastLine).toBe("All steps done.");
    const b = reduceActivity(a, { type: "Stop", last_assistant_message: "TEAMCODE: build done", background_tasks: 0 });
    expect(b.lastLine).toBe("All steps done.");
  });
  it("SubagentStart adds a working member keyed by agent_id and SubagentStop marks it done", () => {
    const a = reduceActivity(emptyActivity, {
      type: "SubagentStart",
      agent_id: "ag-1",
      agent_type: "frontend",
      description: "Step 2 · frontend: cards",
    });
    expect(a.team).toEqual([{ agentId: "ag-1", role: "frontend", working: true }]);
    expect(reduceActivity(a, { type: "SubagentStop", agent_id: "ag-1" }).team[0]!.working).toBe(false);
  });
  it("SubagentStart claims a seeded member of the same role instead of adding a second", () => {
    const seeded = { ...emptyActivity, team: [{ agentId: null, role: "frontend", working: false }] };
    const a = reduceActivity(seeded, {
      type: "SubagentStart",
      agent_id: "ag-1",
      agent_type: "frontend",
      description: null,
    });
    expect(a.team).toEqual([{ agentId: "ag-1", role: "frontend", working: true }]);
  });
  it("SessionStart clears the team and keeps the current file", () => {
    const a = reduceActivity(reduceActivity(emptyActivity, edit("/r/a.ts")), {
      type: "SubagentStart",
      agent_id: "x",
      agent_type: "backend",
      description: null,
    });
    const b = reduceActivity(a, { type: "SessionStart", session_id: "s", transcript_path: "/t" });
    expect(b.team).toEqual([]);
    expect(b.currentFile?.path).toBe("/r/a.ts");
    expect(b.lastLine).toBe("Session started");
  });
  it("PermissionRequest, Notification and StopFailure set the last line", () => {
    expect(reduceActivity(emptyActivity, { type: "PermissionRequest", tool: "Bash" }).lastLine).toBe(
      "Permission needed: Bash",
    );
    expect(reduceActivity(emptyActivity, { type: "Notification", message: "Waiting for input" }).lastLine).toBe(
      "Waiting for input",
    );
    expect(reduceActivity(emptyActivity, { type: "StopFailure", message: "rate limited" }).lastLine).toBe(
      "Error: rate limited",
    );
  });
  it("an event the card does not show leaves the activity untouched", () => {
    expect(reduceActivity(emptyActivity, { type: "UserPromptSubmit" })).toBe(emptyActivity);
    expect(reduceActivity(emptyActivity, { type: "Unknown", name: "X" })).toBe(emptyActivity);
  });
});

describe("activityStore", () => {
  let m: ReturnType<typeof mockIpc>;
  beforeEach(() => {
    activityStore.reset();
    m = mockIpc();
  });
  it("loadActivity seeds the current file, +/- and team from tasks_activity", async () => {
    m.ipc.tasksActivity.mockResolvedValue([
      {
        task_id: "t-1",
        current_file: "/r/src/b.ts",
        plus: 12,
        minus: 3,
        cost_usd: 0.42,
        team: [
          { name: "frontend", state: { type: "Working" } },
          { name: "reviewer", state: { type: "NeedsYou", reason: { type: "StageDone" } } },
        ],
      },
      { task_id: "t-2", current_file: null, plus: 0, minus: 0, cost_usd: null, team: [] },
    ]);
    await loadActivity(projectContainer("p-1"));
    expect(m.ipc.tasksActivity).toHaveBeenCalledWith(projectContainer("p-1"));
    expect(activityStore.get()["t-1"]).toEqual({
      lastLine: null,
      currentFile: { path: "/r/src/b.ts", added: 12, removed: 3 },
      team: [
        { agentId: null, role: "frontend", working: true },
        { agentId: null, role: "reviewer", working: false },
      ],
    });
    expect(activityStore.get()["t-2"]).toEqual(emptyActivity);
  });
  it("a snapshot does not overwrite what live events already said", async () => {
    startActivityEvents();
    m.emit("team-event", { task_id: "t-1", event: makeEvent("t-1", edit("/r/live.ts")) });
    m.ipc.tasksActivity.mockResolvedValue([
      { task_id: "t-1", current_file: "/r/old.ts", plus: 1, minus: 1, cost_usd: null, team: [] },
    ]);
    await loadActivity(projectContainer("p-1"));
    expect(activityStore.get()["t-1"]!.currentFile?.path).toBe("/r/live.ts");
  });
  it("routes team-event payloads to the task", () => {
    startActivityEvents();
    m.emit("team-event", { task_id: "t-1", event: makeEvent("t-1", edit("/r/b.ts")) });
    expect(activityStore.get()["t-1"]!.currentFile?.path).toBe("/r/b.ts");
  });
  it("applyChanges fills +/- for the current file from git_changes", () => {
    activityStore.set({ "t-1": reduceActivity(emptyActivity, edit("/r/src/b.ts")) });
    const file: ChangedFile = {
      path: "src/b.ts",
      status: "Modified",
      added: 12,
      removed: 3,
      binary: false,
      owners: [],
      shared: false,
      source: "Worktree",
    };
    applyChanges("t-1", [{ repo_root: "/r", base: null, files: [file], other: [] }]);
    expect(activityStore.get()["t-1"]!.currentFile).toEqual({ path: "/r/src/b.ts", added: 12, removed: 3 });
  });
  it("git-changes refetches only a task with a current file", async () => {
    activityStore.set({ "t-1": reduceActivity(emptyActivity, edit("/r/src/b.ts")), "t-2": emptyActivity });
    m.ipc.gitChanges.mockResolvedValue([]);
    startActivityEvents();
    m.emit("git-changes", { task_id: "t-2" });
    m.emit("git-changes", { task_id: "t-1" });
    await Promise.resolve();
    expect(m.ipc.gitChanges).toHaveBeenCalledTimes(1);
    expect(m.ipc.gitChanges).toHaveBeenCalledWith("t-1");
  });
  it("a failing git_changes leaves the card as it was", async () => {
    activityStore.set({ "t-1": reduceActivity(emptyActivity, edit("/r/src/b.ts")) });
    m.ipc.gitChanges.mockRejectedValue(new Error("not a repo"));
    startActivityEvents();
    m.emit("git-changes", { task_id: "t-1" });
    await Promise.resolve();
    expect(activityStore.get()["t-1"]!.currentFile?.path).toBe("/r/src/b.ts");
  });
});
