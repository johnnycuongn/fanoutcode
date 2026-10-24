// Board view state (filters, settings, focus, lanes, the last drop) and the selectors that derive columns, swimlanes
// and the mode from it plus the shared task cache. Tasks themselves live in the cache (taskCache.ts).
import { useSyncExternalStore } from "react";
import type { Container, DetailLevel, Project, Stage, Task } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { groupByStage } from "./columns";
import { applyFilters, defaultFilters, isNeedsYou } from "./filters";
import { ipc } from "./ipc";
import { all, track } from "./listen";
import { dropTask, putTask, replaceTasks, resetTaskCache, subscribeTasks, taskMap, type TaskMap } from "./taskCache";
import type { BoardColumn, BoardFilters, BoardMode, BoardSettings, LaneState, Swimlane } from "./types";

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

// ---- mode and membership -------------------------------------------------------------------------------------------

function modeOf(container: Container | null, settings: BoardSettings): BoardMode {
  if (container?.type !== "Workspace") return "single";
  if (settings.projectLayer === "Rail") return "one_project";
  return { Swimlanes: "swimlanes", OneProject: "one_project", Mixed: "mixed" }[settings.workspaceBoard] as BoardMode;
}

export const selectMode = (s: BoardState): BoardMode => modeOf(s.container, s.settings);
export const selectReadOnly = (s: BoardState): boolean => selectMode(s) === "one_project" && s.pickedProject === null;

export function selectDetailLevel(s: BoardState, projectId: string): DetailLevel {
  return s.settings.detailLevelByProject[projectId] ?? s.settings.detailLevel;
}

/** The drop-target lane of a task: its project in the swimlane modes, one shared lane `*` otherwise. */
export function laneOf(s: BoardState, task: Pick<Task, "project_id">): string {
  const mode = selectMode(s);
  return mode === "swimlanes" || mode === "one_project" ? task.project_id : "*";
}

export function inContainer(s: BoardState, task: Pick<Task, "project_id">): boolean {
  return s.container?.type === "Project"
    ? task.project_id === s.container.id
    : s.projectOrder.includes(task.project_id);
}

// ---- memoised selectors --------------------------------------------------------------------------------------------

type Selector<R> = (s: BoardState, tasks: TaskMap) => R;

/** Recompute only when one of the listed inputs changed identity, so `useBoard` snapshots stay stable. */
function memoBy<D extends unknown[], R>(
  deps: (s: BoardState, tasks: TaskMap) => D,
  compute: (...d: D) => R,
): Selector<R> {
  let last: { deps: D; value: R } | null = null;
  return (s, tasks) => {
    const d = deps(s, tasks);
    if (last && d.every((v, i) => Object.is(v, last!.deps[i]))) return last.value;
    const value = compute(...d);
    last = { deps: d, value };
    return value;
  };
}

const selectScoped = memoBy(
  (s, tasks) => [tasks, s.container, s.projectOrder] as const,
  (tasks, container, projectOrder) =>
    Object.values(tasks).filter((t) =>
      container?.type === "Project" ? t.project_id === container.id : projectOrder.includes(t.project_id),
    ),
);

const withoutProjects = (f: BoardFilters): BoardFilters => (f.projects.length === 0 ? f : { ...f, projects: [] });

/** Tasks that pass every filter and the one-project pick. */
export const selectVisibleTasks = memoBy(
  (s, tasks) => [selectScoped(s, tasks), s.filters, selectMode(s), s.pickedProject] as const,
  (scoped, filters, mode, picked) => {
    const visible = applyFilters(scoped, filters);
    return mode === "one_project" && picked !== null ? visible.filter((t) => t.project_id === picked) : visible;
  },
);

export const selectColumns: Selector<BoardColumn[]> = memoBy(
  (s, tasks) => [selectVisibleTasks(s, tasks)] as const,
  (visible) => groupByStage(visible),
);

export const selectSwimlanes: Selector<Swimlane[]> = memoBy(
  (s, tasks) => [selectVisibleTasks(s, tasks), s.lanes, s.projects, s.projectOrder] as const,
  (visible, lanes, projects, projectOrder) =>
    lanes.order
      .filter((id) => projectOrder.includes(id) && projects[id])
      .map((id) => {
        const laneTasks = visible.filter((t) => t.project_id === id);
        return {
          projectId: id,
          name: projects[id]!.name,
          color: projects[id]!.color,
          collapsed: lanes.collapsed.includes(id),
          columns: groupByStage(laneTasks),
          needsYou: laneTasks.filter(isNeedsYou).length,
        };
      }),
);

/** Per project: tasks passing every filter except the project filter, and how many of them need you. */
export const selectProjectCounts: Selector<{ projectId: string; count: number; needsYou: number }[]> = memoBy(
  (s, tasks) => [selectScoped(s, tasks), s.filters, s.projectOrder] as const,
  (scoped, filters, projectOrder) => {
    const visible = applyFilters(scoped, withoutProjects(filters));
    return projectOrder.map((projectId) => {
      const mine = visible.filter((t) => t.project_id === projectId);
      return { projectId, count: mine.length, needsYou: mine.filter(isNeedsYou).length };
    });
  },
);

/** Every Jira key on the board, sorted, for the filter's key list. */
export const selectJiraKeys: Selector<string[]> = memoBy(
  (s, tasks) => [selectScoped(s, tasks)] as const,
  (scoped) => [...new Set(scoped.flatMap((t) => (t.jira_key ? [t.jira_key] : [])))].sort(),
);

/** Non-archived tasks that need the developer, whatever the filters say. */
export const selectNeedsYouCount: Selector<number> = (s, tasks) =>
  selectScoped(s, tasks).filter((t) => !t.archived && isNeedsYou(t)).length;

/** Non-archived, unfiltered tasks in the container: zero means a truly empty board. */
export const selectTotalTaskCount: Selector<number> = (s, tasks) =>
  selectScoped(s, tasks).filter((t) => !t.archived).length;

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

function laneStateFor(prev: LaneState, projectOrder: string[]): LaneState {
  const known = prev.order.filter((id) => projectOrder.includes(id));
  return {
    order: [...known, ...projectOrder.filter((id) => !known.includes(id))],
    collapsed: prev.collapsed.filter((id) => projectOrder.includes(id)),
  };
}

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
    lanes: laneStateFor(
      sameContainer(s.container, container) ? s.lanes : { order: [], collapsed: [] },
      index.projectOrder,
    ),
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
      lanes: laneStateFor(s.lanes, index.projectOrder),
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

/** Moves a swimlane. Task 15 adds persistence. */
export function reorderLanes(from: number, to: number): void {
  boardStore.set((s) => {
    const order = [...s.lanes.order];
    const [moved] = order.splice(from, 1);
    if (moved === undefined || to < 0 || to > order.length) return s;
    order.splice(to, 0, moved);
    return { ...s, lanes: { ...s.lanes, order } };
  });
}
