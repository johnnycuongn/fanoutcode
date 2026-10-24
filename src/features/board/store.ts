// Board view state (filters, settings, focus, lanes, the last drop) and the selectors that derive columns, swimlanes
// and the mode from it plus the shared task cache. Tasks themselves live in the cache (taskCache.ts).
import { useSyncExternalStore } from "react";
import type { Container, Project, Stage, Task } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { defaultFilters } from "./filters";
import { ipc } from "./ipc";
import { laneStorageKey, loadLaneState, moveLane, reconcileLanes, saveLaneState } from "./lanes";
import { all, track } from "./listen";
import { dropTask, putTask, replaceTasks, resetTaskCache, subscribeTasks, taskMap, type TaskMap } from "./taskCache";
import { inContainer, modeOf } from "./selectors";
import type { BoardFilters, BoardSettings, LaneState } from "./types";

export * from "./selectors";

export type BoardState = {
  container: Container | null;
  loaded: boolean;
  projects: Record<string, Project>;
  projectOrder: string[];
  filters: BoardFilters;
  settings: BoardSettings;
  focusedProject: string | null;
  pickedProject: string | null;
  focusedTaskId: string | null;
  renamingTaskId: string | null;
  lanes: LaneState;
  lastDrop: { taskId: string; stage: Stage; lane: string; at: number } | null;
};

const DEFAULT_SETTINGS: BoardSettings = {
  detailLevel: "Simple",
  detailLevelByProject: {},
  costOverview: false,
  workspaceBoard: "Swimlanes",
  projectLayer: "MultiRoot",
};

const INITIAL: BoardState = {
  container: null,
  loaded: false,
  projects: {},
  projectOrder: [],
  filters: defaultFilters,
  settings: DEFAULT_SETTINGS,
  focusedProject: null,
  pickedProject: null,
  focusedTaskId: null,
  renamingTaskId: null,
  lanes: { order: [], collapsed: [] },
  lastDrop: null,
};

export const boardStore = createStore<BoardState>(INITIAL);

export function resetBoardStore(): void {
  boardStore.reset();
  resetTaskCache();
}

const sameContainer = (a: Container | null, b: Container | null): boolean =>
  a !== null && b !== null && a.type === b.type && a.id === b.id;

// ---- reading -------------------------------------------------------------------------------------------------------

function subscribeBoth(listener: () => void): () => void {
  const a = boardStore.subscribe(listener);
  const b = subscribeTasks(listener);
  return () => {
    a();
    b();
  };
}

/** Reads board state and the shared task cache; `selector` must return stable references for unchanged input. */
export function useBoard<S>(selector: (s: BoardState, tasks: TaskMap) => S): S {
  const snapshot = () => selector(boardStore.get(), taskMap());
  return useSyncExternalStore(subscribeBoth, snapshot, snapshot);
}

// ---- writing -------------------------------------------------------------------------------------------------------

export const upsertTask = (task: Task): void => putTask(task);
export const removeTask = (id: string): void => dropTask(id);

function indexProjects(projects: Project[]): Pick<BoardState, "projects" | "projectOrder"> {
  return { projects: Object.fromEntries(projects.map((p) => [p.id, p])), projectOrder: projects.map((p) => p.id) };
}

const sameSettings = (a: BoardSettings, b: BoardSettings): boolean => JSON.stringify(a) === JSON.stringify(b);

const sameProjects = (s: BoardState, projects: Project[]): boolean =>
  projects.length === s.projectOrder.length &&
  projects.every(
    (p, i) => s.projectOrder[i] === p.id && s.projects[p.id]?.name === p.name && s.projects[p.id]?.color === p.color,
  );

async function fetchInto(container: Container): Promise<void> {
  const tasks = await ipc.tasksList(container);
  if (!sameContainer(boardStore.get().container, container)) return; // the board moved on while this was loading
  replaceTasks(tasks);
  boardStore.set({ loaded: true });
}

/** Points the board at a container and replaces the shared cache with its tasks. */
export async function loadBoard(container: Container, projects: Project[]): Promise<void> {
  const index = indexProjects(projects);
  boardStore.set((s) => ({
    ...s,
    ...index,
    container,
    loaded: false,
    lanes: sameContainer(s.container, container)
      ? reconcileLanes(s.lanes, index.projectOrder)
      : loadLaneState(laneStorageKey(container), index.projectOrder),
  }));
  await fetchInto(container);
}

/** Applies the host's props. A user-picked project survives re-configuring with the same focus. */
export function configureBoard(input: {
  settings: BoardSettings;
  focusedProject: string | null;
  projects: Project[];
}): void {
  const focus = input.focusedProject === "all" ? null : input.focusedProject;
  const index = indexProjects(input.projects);
  boardStore.set((s) => {
    if (focus === s.focusedProject && sameSettings(s.settings, input.settings) && sameProjects(s, input.projects)) {
      return s; // the host re-rendered with equal props: nothing to do
    }
    const before = modeOf(s.container, s.settings);
    const after = modeOf(s.container, input.settings);
    const refocused = focus !== s.focusedProject || before !== "one_project";
    return {
      ...s,
      ...index,
      settings: input.settings,
      focusedProject: focus,
      pickedProject: after === "one_project" ? (refocused ? focus : s.pickedProject) : s.pickedProject,
      lanes: reconcileLanes(s.lanes, index.projectOrder),
    };
  });
}

export function setFilters(patch: Partial<BoardFilters>): void {
  boardStore.set((s) => ({ ...s, filters: { ...s.filters, ...patch } }));
}
export const clearFilters = (): void => boardStore.set({ filters: defaultFilters });
export const setPickedProject = (id: string | null): void => boardStore.set({ pickedProject: id });
export const setFocusedTask = (id: string | null): void => boardStore.set({ focusedTaskId: id });
export const setRenaming = (id: string | null): void => boardStore.set({ renamingTaskId: id });

/** Subscribes to `task-changed` and `tasks-reloaded`. Every window hears every event: foreign tasks are dropped. */
export function startBoardEvents(): () => void {
  return all(
    track(
      ipc.on("task-changed", ({ task }) => {
        if (inContainer(boardStore.get(), task)) putTask(task);
      }),
    ),
    track(
      ipc.on("tasks-reloaded", ({ container }) => {
        const mine = boardStore.get().container;
        if (mine && sameContainer(mine, container)) void fetchInto(mine);
      }),
    ),
  );
}

function setLanes(update: (lanes: LaneState) => LaneState): void {
  boardStore.set((s) => {
    const lanes = update(s.lanes);
    if (s.container) saveLaneState(laneStorageKey(s.container), lanes);
    return { ...s, lanes };
  });
}

/** Moves a swimlane (indexes into the lane order) and remembers the new order. */
export const reorderLanes = (from: number, to: number): void =>
  setLanes((lanes) => ({ ...lanes, order: moveLane(lanes.order, from, to) }));

export const toggleLane = (projectId: string): void =>
  setLanes((lanes) => ({
    ...lanes,
    collapsed: lanes.collapsed.includes(projectId)
      ? lanes.collapsed.filter((id) => id !== projectId)
      : [...lanes.collapsed, projectId],
  }));
