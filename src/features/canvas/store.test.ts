import { describe, it, expect, vi, beforeEach } from "vitest";
import { fx, sampleBuild, samplePlan, ROOTS, MEMORY_DIR, TASK_ID } from "./fixtures";

const { listeners, commands } = vi.hoisted(() => {
  const listeners: Record<string, (p: { payload: unknown }) => void> = {};
  const commands = {
    tasksGet: vi.fn(async () => ({
      id: "t-0c4",
      project_id: "p-a",
      workspace_id: null,
      stage: "Build",
      state: { type: "Working" },
      memory_dir: "/mem/2026-10-24-canvas",
      linked_roots: [] as string[],
      agent_kind: "ClaudeCode",
    })),
    projectsList: vi.fn(async () => [{ id: "p-a", name: "app", root: "/repo/app", color: "indigo" }]),
    settingsGet: vi.fn(async () => ({
      detail_level: "Simple",
      cost_overview: false,
      team_view_arrangement: "HubGraph",
      team_view_placement: "DockGrows",
      team_canvas_layout: "AutoReflow",
      team_drill_in_lens: "ByFile",
    })),
    hooksList: vi.fn(async (): Promise<unknown[]> => []),
    hooksPlanSteps: vi.fn(async (): Promise<unknown> => ({ steps: [], team: [] })), // PlanDoc
    gitChanges: vi.fn(async () => []),
    costSummary: vi.fn(async () => ({ by_agent: [] })),
    canvasPositionsGet: vi.fn(async () => []),
    canvasPositionsSet: vi.fn<(taskId: string, positions: unknown[]) => Promise<null>>(async () => null),
  };
  return { listeners, commands };
});
vi.mock("../../lib/bindings", () => ({
  commands,
  events: Object.fromEntries(
    ["teamEvent", "taskChanged", "gitChanges", "costUpdated"].map((n) => [
      n,
      {
        listen: async (cb: (p: { payload: unknown }) => void) => {
          listeners[n] = cb;
          return () => delete listeners[n];
        },
      },
    ]),
  ),
}));
vi.mock("./layout", () => ({
  layoutGraph: vi.fn(async (graph: { nodes: { id: string }[] }) =>
    graph.nodes.map((n, i) => ({ node_id: n.id, x: i * 100, y: 0 })),
  ),
}));
const { openCanvas, closeCanvas, getCanvas, moveNode, setReplaySeq, tidy } = await import("./store");

// The plan as Plan 05's PlanDoc: steps 3 (after 1) and 4 (after 2, 3) under the `api` and home headings.
const PLAN_DOC = {
  steps: samplePlan().map((s) => ({
    number: s.number,
    title: s.title,
    done: false,
    deps: s.after,
    explicit_deps: s.after.length > 0,
    repo: s.repo === "/repo/api" ? "api" : null,
  })),
  team: [],
};

beforeEach(() => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  commands.hooksList.mockImplementation(async () => sampleBuild().slice(0, 10));
  commands.hooksPlanSteps.mockImplementation(async () => PLAN_DOC);
});

describe("canvas store", () => {
  it("openCanvas loads steps, settings, events and positions and derives a graph", async () => {
    await openCanvas(TASK_ID);
    await vi.runAllTimersAsync();
    const s = getCanvas(TASK_ID);
    expect(s.graph.nodes.length).toBe(3 + 6 + 2);
    expect(s.positions.length).toBe(s.graph.nodes.length);
    expect(s.task?.memory_dir).toBe(MEMORY_DIR);
    expect(s.roots.map((r) => r.root)).toEqual([ROOTS[0].root]);
    closeCanvas(TASK_ID);
  });
  it("applies a team-event for this task and ignores one for another task", async () => {
    await openCanvas(TASK_ID);
    listeners.teamEvent({ payload: { task_id: "t-0ff", event: fx.edit(99, "zz", "/repo/app/z.ts", "", "z") } });
    expect(getCanvas(TASK_ID).events).toHaveLength(10);
    listeners.teamEvent({ payload: { task_id: TASK_ID, event: fx.edit(11, "a2", "/repo/app/src/new.ts", "", "n") } });
    expect(getCanvas(TASK_ID).model.files.map((f) => f.path)).toContain("/repo/app/src/new.ts");
    closeCanvas(TASK_ID);
  });
  it("persists positions once, 500 ms after a drag settles, and only for current nodes", async () => {
    await openCanvas(TASK_ID);
    await vi.runAllTimersAsync();
    commands.canvasPositionsSet.mockClear();
    moveNode(TASK_ID, "step:1", 5, 5);
    moveNode(TASK_ID, "step:1", 7, 7);
    await vi.advanceTimersByTimeAsync(499);
    expect(commands.canvasPositionsSet).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(commands.canvasPositionsSet).toHaveBeenCalledTimes(1);
    const saved = commands.canvasPositionsSet.mock.calls[0][1] as { node_id: string }[];
    expect(saved.find((p) => p.node_id === "step:1")).toMatchObject({ x: 7, y: 7 });
    expect(saved.every((p) => getCanvas(TASK_ID).graph.nodes.some((n) => n.id === p.node_id))).toBe(true);
    closeCanvas(TASK_ID);
  });
  it("pinned: a new node triggers a layout but a dragged node stays where it was", async () => {
    commands.settingsGet.mockResolvedValueOnce({ ...(await commands.settingsGet()), team_canvas_layout: "Pinned" });
    await openCanvas(TASK_ID);
    await vi.runAllTimersAsync();
    moveNode(TASK_ID, "step:1", 500, 500);
    listeners.teamEvent({ payload: { task_id: TASK_ID, event: fx.edit(11, "a2", "/repo/app/src/new.ts", "", "n") } });
    await vi.runAllTimersAsync();
    const s = getCanvas(TASK_ID);
    expect(s.positions.find((p) => p.node_id === "step:1")).toMatchObject({ x: 500, y: 500 });
    expect(s.positions.some((p) => p.node_id === "file:/repo/app/src/new.ts")).toBe(true);
    closeCanvas(TASK_ID);
  });
  it("auto re-flow: a dragged node snaps back when the graph changes", async () => {
    await openCanvas(TASK_ID);
    await vi.runAllTimersAsync();
    moveNode(TASK_ID, "step:1", 500, 500);
    listeners.teamEvent({ payload: { task_id: TASK_ID, event: fx.edit(11, "a2", "/repo/app/src/new.ts", "", "n") } });
    await vi.runAllTimersAsync();
    expect(getCanvas(TASK_ID).positions.find((p) => p.node_id === "step:1")).not.toMatchObject({ x: 500, y: 500 });
    closeCanvas(TASK_ID);
  });
  it("replay cursor derives from the truncated stream", async () => {
    await openCanvas(TASK_ID);
    setReplaySeq(TASK_ID, 8);
    expect(getCanvas(TASK_ID).model.files.find((f) => f.path === "/repo/app/src/a.ts")?.sharedAt).toBeNull();
    setReplaySeq(TASK_ID, null);
    expect(getCanvas(TASK_ID).model.files.find((f) => f.path === "/repo/app/src/a.ts")?.sharedAt).toBe(9);
    closeCanvas(TASK_ID);
  });
  it("tidy runs a layout at once in pinned mode", async () => {
    commands.settingsGet.mockResolvedValueOnce({ ...(await commands.settingsGet()), team_canvas_layout: "Pinned" });
    await openCanvas(TASK_ID);
    await vi.runAllTimersAsync();
    moveNode(TASK_ID, "step:1", 500, 500);
    await tidy(TASK_ID);
    expect(getCanvas(TASK_ID).positions.find((p) => p.node_id === "step:1")).not.toMatchObject({ x: 500, y: 500 });
    closeCanvas(TASK_ID);
  });
});
