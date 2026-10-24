// The tasks of this window's container, shared by the shell, the board (Plan 07) and the task page (Plan 08), plus
// the needs-you count of every container the app has reported. Fed by `tasks_list` and the `task-changed`,
// `tasks-reloaded` and `needs-you` events. This module is also the only code that builds a `Container`.
import {
  commands,
  events,
  type AgentState,
  type Container,
  type ProjectId,
  type Task,
  type TaskId,
  type Workspace,
  type WorkspaceId,
} from "@/lib/bindings";
import { createStore } from "./createStore";

export type TasksState = {
  container: Container | null;
  byId: Record<TaskId, Task>;
  order: TaskId[];
  needsYouByContainer: Record<string, number>;
  loaded: boolean;
};

export const tasksStore = createStore<TasksState>({
  container: null,
  byId: {},
  order: [],
  needsYouByContainer: {},
  loaded: false,
});

export function projectContainer(id: ProjectId): Container {
  return { type: "Project", id };
}

export function workspaceContainer(id: WorkspaceId): Container {
  return { type: "Workspace", id };
}

export function containerKey(c: Container): string {
  return c.type === "Project" ? `project:${c.id}` : `workspace:${c.id}`;
}

export function sameContainer(a: Container | null, b: Container | null): boolean {
  return a !== null && b !== null && containerKey(a) === containerKey(b);
}

export function containerProjectIds(c: Container, workspaces: Workspace[]): ProjectId[] {
  if (c.type === "Project") return [c.id];
  const ws = workspaces.find((w) => w.id === c.id);
  return ws ? [...ws.projects].sort((a, b) => a.order - b.order).map((p) => p.project_id) : [];
}

// The projects of a workspace container, as the window reported them; a task of one of them belongs here.
let containerProjects = new Set<ProjectId>();

/** The window's project ids (WindowInfo.project_ids); used to place `task-changed` tasks in a workspace. */
export function setContainerProjects(ids: ProjectId[]): void {
  containerProjects = new Set(ids);
}

function belongs(container: Container, task: Task): boolean {
  if (container.type === "Project") return task.project_id === container.id;
  return task.workspace_id === container.id || containerProjects.has(task.project_id);
}

export async function loadTasks(container: Container): Promise<void> {
  const r = await commands.tasksList(container);
  if (r.status !== "ok") return;
  const byId: Record<TaskId, Task> = {};
  for (const t of r.data) byId[t.id] = t;
  tasksStore.set({ container, byId, order: r.data.map((t) => t.id), loaded: true });
}

/** Upserts a task of this window's container; tasks of other containers are ignored. */
export function applyTaskChanged(task: Task): void {
  const { container } = tasksStore.get();
  if (!container || !belongs(container, task)) return;
  tasksStore.set((s) => ({
    ...s,
    byId: { ...s.byId, [task.id]: task },
    order: s.byId[task.id] ? s.order : [...s.order, task.id],
  }));
}

export function applyNeedsYou(container: Container, count: number): void {
  const key = containerKey(container);
  if (tasksStore.get().needsYouByContainer[key] === count) return;
  tasksStore.set((s) => ({ ...s, needsYouByContainer: { ...s.needsYouByContainer, [key]: count } }));
}

/** Subscribes `task-changed`, `tasks-reloaded` and `needs-you`; returns the unsubscribe. */
export function startTasksEvents(): () => void {
  const offs = [
    events.taskChanged.listen((e) => applyTaskChanged(e.payload.task)),
    events.tasksReloaded.listen((e) => {
      const { container } = tasksStore.get();
      if (sameContainer(container, e.payload.container)) void loadTasks(e.payload.container);
    }),
    events.needsYou.listen((e) => applyNeedsYou(e.payload.container, e.payload.count)),
  ];
  return () => {
    for (const off of offs) void off.then((fn) => fn());
  };
}

export function isNeedsYou(state: AgentState): boolean {
  return state.type === "NeedsYou";
}

const newestFirst = (a: Task, b: Task) => b.updated_at.localeCompare(a.updated_at);

/** Non-archived tasks grouped by agent state, each group newest first. */
export function tasksByState(tasks: Task[]): { needsYou: Task[]; running: Task[]; idle: Task[] } {
  const live = tasks.filter((t) => !t.archived).sort(newestFirst);
  return {
    needsYou: live.filter((t) => t.state.type === "NeedsYou"),
    running: live.filter((t) => t.state.type === "Working"),
    idle: live.filter((t) => t.state.type === "Idle"),
  };
}

/** Non-archived needs-you tasks, newest first; only `projectId`'s when given. */
export function needsYouTasks(tasks: Task[], projectId?: ProjectId): Task[] {
  return tasks
    .filter((t) => !t.archived && isNeedsYou(t.state) && (projectId === undefined || t.project_id === projectId))
    .sort(newestFirst);
}

/** The container's tasks in load order (a stable array per state, for `useSelector`). */
let listCache: { byId: Record<TaskId, Task>; order: TaskId[]; list: Task[] } | null = null;
export function selectTaskList(s: TasksState): Task[] {
  if (listCache && listCache.byId === s.byId && listCache.order === s.order) return listCache.list;
  const list = s.order.map((id) => s.byId[id]).filter((t): t is Task => Boolean(t));
  listCache = { byId: s.byId, order: s.order, list };
  return list;
}
