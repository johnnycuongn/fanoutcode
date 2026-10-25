import { expect, test, vi, type Mock } from "vitest";
import type { AgentState, Task, TaskEvent } from "@/lib/bindings";
import { createTaskPageStore } from "./store";
import { makeTask, makeTaskSource } from "./testing/fixtures";
import type { TaskPageIpc } from "./types";

const resolved = {
  may_commit: false,
  cost_overview: true,
  test_command: "npm test",
  advance_on_green: false,
  dock_position: "Bottom",
  team_view_placement: "DockGrows",
};
const working: AgentState = { type: "Working" };
function fakeIpc(task = makeTask()): TaskPageIpc {
  return {
    tasksGet: vi.fn(async () => task),
    tasksAction: vi.fn(async () => {}),
    tasksRename: vi.fn(async () => {}),
    sessionsStop: vi.fn(async () => {}),
    sessionsOpen: vi.fn(async () => "sess-1"),
    costSummary: vi.fn(async () => ({ cost_usd: 0.5, input_tokens: 100, output_tokens: 20, estimated: false })),
    settingsGet: vi.fn(async () => resolved),
  } as unknown as TaskPageIpc;
}
function setup(task = makeTask(), shared: Task[] = [task]) {
  const ipc = fakeIpc(task);
  const src = makeTaskSource(shared);
  return { ipc, src, s: createTaskPageStore(ipc, src) };
}

test("open reads the shared task and its resolved settings without refetching", async () => {
  const { ipc, s } = setup();
  await expect(s.open("task-1")).resolves.toMatchObject({ id: "task-1" });
  expect(ipc.tasksGet).not.toHaveBeenCalled();
  expect(s.store.get().settings["task-1"]).toEqual({
    mayCommit: false,
    costOverview: true,
    testCommand: "npm test",
    advanceOnGreen: false,
    dockPosition: "Bottom",
    teamViewPlacement: "DockGrows",
  });
});
test("open fetches a task the shared store lacks and puts it there", async () => {
  const { ipc, src, s } = setup(makeTask(), []);
  await s.open("task-1");
  expect(ipc.tasksGet).toHaveBeenCalledWith("task-1");
  expect(src.get("task-1")?.id).toBe("task-1");
});
test("act while Working marks the action queued until the shared task brings a new state", async () => {
  const { ipc, src, s } = setup(makeTask({ state: working }));
  await s.open("task-1");
  await s.act("task-1", { type: "Advance" });
  expect(ipc.tasksAction).toHaveBeenCalledWith("task-1", { type: "Advance" });
  expect(s.store.get().queued["task-1"]).toEqual({ type: "Advance" });
  src.put(makeTask({ state: working }));
  expect(s.store.get().queued["task-1"]).toEqual({ type: "Advance" });
  src.put(makeTask({ stage: "Build", state: working }));
  expect(s.store.get().queued["task-1"]).toBeUndefined();
});
test("runPrimary in the Replied state focuses the terminal and sends nothing", async () => {
  const { ipc, s } = setup(makeTask({ state: { type: "NeedsYou", reason: { type: "Replied", first_line: "?" } } }));
  await s.open("task-1");
  const focus = vi.fn();
  await s.runPrimary("task-1", focus);
  expect(focus).toHaveBeenCalled();
  expect(ipc.tasksAction).not.toHaveBeenCalled();
});
test("runPrimary otherwise sends the stage's action", async () => {
  const { ipc, s } = setup();
  await s.open("task-1");
  await s.runPrimary("task-1", () => {});
  expect(ipc.tasksAction).toHaveBeenCalledWith("task-1", { type: "Advance" });
});
test("cost-updated refetches the task's cost summary", async () => {
  const { ipc, s } = setup();
  await s.open("task-1");
  await s.onCostUpdated("task-1");
  expect(ipc.costSummary).toHaveBeenCalledWith(expect.objectContaining({ task: "task-1" }));
  expect(s.store.get().cost["task-1"].cost_usd).toBe(0.5);
});
test("a StatusLine team-event updates the context meter", async () => {
  const { s } = setup();
  await s.open("task-1");
  s.onTeamEvent("task-1", {
    seq: 1,
    at: "2026-10-24T05:12:00Z",
    task_id: "task-1",
    agent_id: null,
    event: { type: "StatusLine", cost_usd: 1, input_tokens: 1, output_tokens: 1, context_used_pct: 73.5 },
  } as TaskEvent);
  expect(s.store.get().contextPct["task-1"]).toBe(73.5);
});
test("the live cost line also feeds the context meter", async () => {
  const { ipc, s } = setup();
  (ipc.costSummary as Mock).mockResolvedValue({
    cost_usd: 1,
    input_tokens: 1,
    output_tokens: 1,
    estimated: false,
    live: { cost_usd: 1, context_used_pct: 41, at: "" },
  });
  await s.open("task-1");
  expect(s.store.get().contextPct["task-1"]).toBe(41);
});
test("rename sends tasks_rename; a rejected action surfaces its message", async () => {
  const { ipc, s } = setup();
  (ipc.tasksAction as Mock).mockRejectedValueOnce("Session is not at its prompt");
  await s.open("task-1");
  await s.rename("task-1", "New name");
  await s.act("task-1", { type: "Advance" });
  expect(ipc.tasksRename).toHaveBeenCalledWith("task-1", "New name");
  expect(s.store.get().errors["task-1"]).toBe("Session is not at its prompt");
});
test("an action's returned task replaces the shared copy", async () => {
  const { ipc, src, s } = setup();
  (ipc.tasksAction as Mock).mockResolvedValueOnce({ task: makeTask({ stage: "Build" }), toasts: [] });
  await s.open("task-1");
  await s.act("task-1", { type: "Advance" });
  expect(src.get("task-1")?.stage).toBe("Build");
});
test("stopAgent stops the primary session", async () => {
  const { ipc, s } = setup();
  await s.open("task-1");
  await s.stopAgent("task-1");
  expect(ipc.sessionsStop).toHaveBeenCalledWith("sess-1");
});
