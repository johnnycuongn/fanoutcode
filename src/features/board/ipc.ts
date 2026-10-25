// The only board file that touches the generated bindings: every IPC call and event the board uses goes through
// `ipc`, so tests mock this one module and the binding names live in one place. `unwrap` turns a specta `Result`
// into a thrown `Error`.
import {
  commands,
  events,
  type ActionOutcome,
  type Container,
  type CostSummary,
  type CostUpdated,
  type GitChanges,
  type RepoChanges,
  type Result,
  type Scope,
  type Stage,
  type Task,
  type TaskActivity,
  type TaskChanged,
  type TaskPatch,
  type TasksReloaded,
  type TeamEvent,
  type WindowInfo,
} from "@/lib/bindings";
import type { TaskPatchInput } from "./types";

export type Unlisten = () => void;

export type EventMap = {
  "task-changed": TaskChanged;
  "tasks-reloaded": TasksReloaded;
  "team-event": TeamEvent;
  "cost-updated": CostUpdated;
  "git-changes": GitChanges;
};
export type EventName = keyof EventMap;

export function unwrap<T>(result: Result<T, string> | T): T {
  if (typeof result === "object" && result !== null && "status" in result) {
    const r = result as Result<T, string>;
    if (r.status === "ok") return r.data;
    if (r.status === "error") throw new Error(r.error);
  }
  return result as T;
}

function toPatch(p: TaskPatchInput): TaskPatch {
  return {
    priority: p.priority ?? null,
    color: p.color ?? null,
    clear_color: p.color === null,
    flagged: p.flagged ?? null,
    hidden: p.hidden ?? null,
    jira_key: p.jira_key ?? null,
    clear_jira_key: p.jira_key === null,
    worktree: p.worktree ?? null,
    agent_kind: null,
  };
}

function listen<K extends EventName>(name: K, cb: (payload: EventMap[K]) => void): Promise<Unlisten> {
  const handler = cb as (payload: unknown) => void;
  switch (name) {
    case "task-changed":
      return events.taskChanged.listen((e) => handler(e.payload));
    case "tasks-reloaded":
      return events.tasksReloaded.listen((e) => handler(e.payload));
    case "cost-updated":
      return events.costUpdated.listen((e) => handler(e.payload));
    case "git-changes":
      return events.gitChanges.listen((e) => handler(e.payload));
    case "team-event":
      return events.teamEvent.listen((e) => handler(e.payload));
  }
  throw new Error(`Unknown event ${String(name)}`);
}

export const ipc = {
  async tasksList(container: Container): Promise<Task[]> {
    return unwrap(await commands.tasksList(container));
  },
  async tasksCreate(
    projectId: string,
    intent: string,
    jiraKey: string | null,
    workspaceId: string | null,
  ): Promise<Task> {
    return unwrap(await commands.tasksCreate(projectId, intent, jiraKey, workspaceId));
  },
  async tasksMove(id: string, stage: Stage): Promise<ActionOutcome> {
    return unwrap(await commands.tasksMove(id, stage));
  },
  async tasksUpdate(id: string, patch: TaskPatchInput): Promise<Task> {
    return unwrap(await commands.tasksUpdate(id, toPatch(patch)));
  },
  async tasksRename(id: string, title: string): Promise<Task> {
    return unwrap(await commands.tasksRename(id, title));
  },
  async tasksArchive(id: string): Promise<ActionOutcome> {
    return unwrap(await commands.tasksArchive(id));
  },
  async tasksRestore(id: string): Promise<ActionOutcome> {
    return unwrap(await commands.tasksRestore(id));
  },
  async sessionsStop(sessionId: string): Promise<void> {
    unwrap(await commands.sessionsStop(sessionId));
  },
  async windowsList(): Promise<WindowInfo[]> {
    return unwrap(await commands.windowsList());
  },
  async gitChanges(taskId: string): Promise<RepoChanges[]> {
    return unwrap(await commands.gitChanges(taskId));
  },
  async costSummary(scope: Scope): Promise<CostSummary> {
    return unwrap(await commands.costSummary(scope));
  },
  async tasksActivity(container: Container): Promise<TaskActivity[]> {
    return unwrap(await commands.tasksActivity(container));
  },
  on: listen,
};
