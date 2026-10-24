// Pure selectors over the board state and the shared task cache. Each one that returns a list or an object is memoised
// on the identity of its inputs, so `useBoard` snapshots stay stable and a card re-renders only when its data changes.
import type { Container, DetailLevel, Task } from "@/lib/bindings";
import { groupByStage } from "./columns";
import { applyFilters, isNeedsYou } from "./filters";
import type { BoardState } from "./store";
import type { TaskMap } from "./taskCache";
import type { BoardColumn, BoardFilters, BoardMode, BoardSettings, Swimlane } from "./types";

export function modeOf(container: Container | null, settings: BoardSettings): BoardMode {
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
