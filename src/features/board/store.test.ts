import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("./ipc");
import { mockIpc } from "./testing/mockIpc";
import { makeProject, makeTask, needsYou, projectContainer, workspaceContainer } from "./testing/fixtures";
import { taskMap } from "./taskCache";
import {
  boardStore,
  configureBoard,
  loadBoard,
  resetBoardStore,
  selectColumns,
  selectMode,
  selectProjectCounts,
  selectReadOnly,
  selectSwimlanes,
  setFilters,
  setPickedProject,
  startBoardEvents,
  upsertTask,
} from "./store";

const settings = {
  detailLevel: "Simple",
  detailLevelByProject: {},
  costOverview: false,
  workspaceBoard: "Swimlanes",
  projectLayer: "MultiRoot",
} as const;
const p1 = makeProject({ id: "p-1", name: "Alpha" });
const p2 = makeProject({ id: "p-2", name: "Beta" });
const workspace = workspaceContainer("ws-1");
const project = projectContainer("p-1");

describe("boardStore", () => {
  let m: ReturnType<typeof mockIpc>;
  beforeEach(() => {
    resetBoardStore();
    m = mockIpc();
  });

  it("loadBoard fills the shared task cache from tasks_list and marks loaded", async () => {
    m.ipc.tasksList.mockResolvedValue([makeTask({ id: "t-1" })]);
    await loadBoard(project, [p1]);
    expect(m.ipc.tasksList).toHaveBeenCalledWith(project);
    expect(Object.keys(taskMap())).toEqual(["t-1"]);
    expect(boardStore.get().loaded).toBe(true);
  });
  it("ignores a task-changed for a task outside the container", async () => {
    // Review Focus 1
    await loadBoard(project, [p1]);
    startBoardEvents();
    m.emit("task-changed", { task: makeTask({ id: "foreign", project_id: "p-7" }) });
    expect(taskMap().foreign).toBeUndefined();
  });
  it("upserts a task-changed for a task in the container", async () => {
    await loadBoard(workspace, [p1, p2]);
    startBoardEvents();
    m.emit("task-changed", { task: makeTask({ id: "t-2", project_id: "p-2" }) });
    expect(taskMap()["t-2"]!.project_id).toBe("p-2");
  });
  it("reloads on tasks-reloaded for the same container only", async () => {
    await loadBoard(project, [p1]);
    startBoardEvents();
    m.emit("tasks-reloaded", { container: projectContainer("p-9") });
    m.emit("tasks-reloaded", { container: project });
    expect(m.ipc.tasksList).toHaveBeenCalledTimes(2);
  });
  it("a slow load for a container the board has left does not overwrite the new one", async () => {
    let release: (tasks: ReturnType<typeof makeTask>[]) => void = () => {};
    m.ipc.tasksList.mockReturnValueOnce(new Promise((resolve) => (release = resolve)));
    const stale = loadBoard(project, [p1]);
    m.ipc.tasksList.mockResolvedValueOnce([makeTask({ id: "fresh", project_id: "p-2" })]);
    await loadBoard(projectContainer("p-2"), [p2]);
    release([makeTask({ id: "stale" })]);
    await stale;
    expect(Object.keys(taskMap())).toEqual(["fresh"]);
  });
  it("selectColumns applies filters and returns the same reference when nothing changed", () => {
    configureBoard({ settings, focusedProject: null, projects: [p1] });
    upsertTask(makeTask({ id: "a", stage: "Plan" }));
    upsertTask(makeTask({ id: "b", stage: "Plan", state: needsYou({ type: "StageDone" }) }));
    const first = selectColumns(boardStore.get(), taskMap());
    expect(first[1]!.count).toBe(2);
    expect(selectColumns(boardStore.get(), taskMap())).toBe(first);
    setFilters({ needsYou: true });
    expect(selectColumns(boardStore.get(), taskMap())[1]!.tasks.map((t) => t.id)).toEqual(["b"]);
  });
  it("selectMode is single for a project and follows workspace_board for a workspace", async () => {
    await loadBoard(project, [p1]);
    configureBoard({ settings, focusedProject: null, projects: [p1] });
    expect(selectMode(boardStore.get())).toBe("single");
    await loadBoard(workspace, [p1, p2]);
    expect(selectMode(boardStore.get())).toBe("swimlanes");
    configureBoard({ settings: { ...settings, workspaceBoard: "Mixed" }, focusedProject: null, projects: [p1, p2] });
    expect(selectMode(boardStore.get())).toBe("mixed");
  });
  it("selectMode is one_project under the Rail project layer whatever workspace_board says", async () => {
    await loadBoard(workspace, [p1, p2]);
    configureBoard({
      settings: { ...settings, projectLayer: "Rail" },
      focusedProject: "p-2",
      projects: [p1, p2],
    });
    expect(selectMode(boardStore.get())).toBe("one_project");
    expect(boardStore.get().pickedProject).toBe("p-2");
  });
  it("re-configuring with the same focus keeps a project the user picked", async () => {
    await loadBoard(workspace, [p1, p2]);
    const one = { ...settings, workspaceBoard: "OneProject" } as const;
    configureBoard({ settings: one, focusedProject: null, projects: [p1, p2] });
    setPickedProject("p-2");
    configureBoard({ settings: { ...one }, focusedProject: null, projects: [p1, p2] });
    expect(boardStore.get().pickedProject).toBe("p-2");
    configureBoard({ settings: one, focusedProject: "p-1", projects: [p1, p2] });
    expect(boardStore.get().pickedProject).toBe("p-1");
  });
  it("treats the host's 'all' focus as no focus", async () => {
    await loadBoard(workspace, [p1, p2]);
    configureBoard({
      settings: { ...settings, projectLayer: "Rail" },
      focusedProject: "all",
      projects: [p1, p2],
    });
    expect(boardStore.get().pickedProject).toBeNull();
  });
  it("selectSwimlanes yields one lane per project in order with a needsYou count", async () => {
    m.ipc.tasksList.mockResolvedValue([
      makeTask({ id: "a", project_id: "p-2", state: needsYou({ type: "StageDone" }) }),
    ]);
    await loadBoard(workspace, [p1, p2]);
    configureBoard({ settings, focusedProject: null, projects: [p1, p2] });
    const lanes = selectSwimlanes(boardStore.get(), taskMap());
    expect(lanes.map((l) => [l.projectId, l.name, l.needsYou])).toEqual([
      ["p-1", "Alpha", 0],
      ["p-2", "Beta", 1],
    ]);
  });
  it("selectProjectCounts ignores the project filter", async () => {
    m.ipc.tasksList.mockResolvedValue([
      makeTask({ id: "a", project_id: "p-1" }),
      makeTask({ id: "b", project_id: "p-2" }),
    ]);
    await loadBoard(workspace, [p1, p2]);
    configureBoard({ settings, focusedProject: null, projects: [p1, p2] });
    setFilters({ projects: ["p-1"] });
    expect(selectProjectCounts(boardStore.get(), taskMap()).map((c) => c.count)).toEqual([1, 1]);
  });
  it("selectReadOnly is true in one_project mode with All picked", async () => {
    await loadBoard(workspace, [p1, p2]);
    configureBoard({
      settings: { ...settings, workspaceBoard: "OneProject" },
      focusedProject: null,
      projects: [p1, p2],
    });
    expect(selectReadOnly(boardStore.get())).toBe(true);
    setPickedProject("p-1");
    expect(selectReadOnly(boardStore.get())).toBe(false);
  });
});
