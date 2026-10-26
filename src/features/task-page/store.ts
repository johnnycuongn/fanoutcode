// Page-local state for open task pages: the stage and state last seen (to clear a queued action), the resolved
// settings, the queued action while the agent works, the task's cost and context meter, and the last error.
import { toast } from "@/components/ui";
import type { ActionOutcome, ResolvedSettings, Scope, StageAction, Task, TaskEvent, TaskId } from "@/lib/bindings";
import { errorMessage, live } from "@/lib/ipcResult";
import { createStore } from "@/lib/stores/createStore";
import type { DockTab } from "@/features/dock";
import { isWorking, sameState } from "./agentState";
import { primaryActionFor } from "./primaryAction";
import { tasksSource, type TaskSource } from "./taskSource";
import type { TaskPageIpc, TaskPageState, TaskSettings } from "./types";

export const DEFAULT_TASK_SETTINGS: TaskSettings = {
  mayCommit: false,
  costOverview: false,
  testCommand: null,
  testEnabled: true,
  advanceOnGreen: false,
  dockPosition: "Bottom",
  teamViewPlacement: "DockGrows",
};

export function toTaskSettings(r: ResolvedSettings): TaskSettings {
  return {
    mayCommit: r.may_commit ?? false,
    costOverview: r.cost_overview ?? false,
    testCommand: r.test_command ?? null,
    testEnabled: r.test_enabled ?? true,
    advanceOnGreen: r.advance_on_green ?? false,
    dockPosition: r.dock_position ?? "Bottom",
    teamViewPlacement: r.team_view_placement ?? "DockGrows",
  };
}

export const scopeOf = (t: Task): Scope => ({ workspace: t.workspace_id, project: t.project_id, task: t.id });

const initial: TaskPageState = { seen: {}, settings: {}, queued: {}, cost: {}, contextPct: {}, errors: {} };

function without<T>(rec: Record<string, T>, key: string): Record<string, T> {
  if (!(key in rec)) return rec;
  const next = { ...rec };
  delete next[key];
  return next;
}

const isOutcome = (r: unknown): r is ActionOutcome => typeof r === "object" && r !== null && "task" in r;

export function createTaskPageStore(ipc: TaskPageIpc, tasks: TaskSource = tasksSource) {
  const store = createStore<TaskPageState>(initial);
  const put = <K extends keyof TaskPageState>(key: K, id: string, value: TaskPageState[K][string]) =>
    store.set((s) => ({ ...s, [key]: { ...s[key], [id]: value } }));
  const drop = (key: keyof TaskPageState, id: string) =>
    store.set((s) => (id in s[key] ? { ...s, [key]: without(s[key] as Record<string, unknown>, id) } : s));

  // The shared task moved on: a queued action has been taken up (or overtaken), and the page remembers the new pair.
  tasks.subscribe(() => {
    for (const [id, seen] of Object.entries(store.get().seen)) {
      const t = tasks.get(id);
      if (!t || (t.stage === seen.stage && sameState(t.state, seen.state))) continue;
      store.set((s) => ({
        ...s,
        seen: { ...s.seen, [id]: { stage: t.stage, state: t.state } },
        queued: without(s.queued, id),
      }));
    }
  });

  const fail = (taskId: TaskId, e: unknown) => {
    const message = errorMessage(e);
    put("errors", taskId, message);
    toast.error(message);
  };

  async function onCostUpdated(taskId: TaskId): Promise<void> {
    const task = tasks.get(taskId);
    if (!task) return;
    try {
      const cost = await ipc.costSummary(scopeOf(task));
      put("cost", taskId, cost);
      if (cost.live) put("contextPct", taskId, cost.live.context_used_pct);
    } catch {
      // keep the last numbers; the meter is informational
    }
  }

  async function open(taskId: TaskId): Promise<Task> {
    let task = tasks.get(taskId);
    if (!task) {
      task = await ipc.tasksGet(taskId);
      tasks.put(task);
    }
    put("seen", taskId, { stage: task.stage, state: task.state });
    try {
      const settings = toTaskSettings(await ipc.settingsGet(scopeOf(task)));
      put("settings", taskId, settings);
      if (settings.costOverview) await onCostUpdated(taskId);
    } catch (e) {
      put("errors", taskId, errorMessage(e));
    }
    return task;
  }

  async function act(taskId: TaskId, action: StageAction): Promise<void> {
    const task = tasks.get(taskId);
    if (task && isWorking(task.state)) put("queued", taskId, action);
    drop("errors", taskId);
    try {
      const outcome = await ipc.tasksAction(taskId, action);
      if (isOutcome(outcome)) {
        tasks.put(outcome.task);
        for (const t of outcome.toasts) toast.info(t);
      }
    } catch (e) {
      drop("queued", taskId);
      fail(taskId, e);
    }
  }

  return {
    store,
    open,
    act,
    async runPrimary(taskId: TaskId, focusTerminal: () => void, openDockTab?: (tab: DockTab) => void): Promise<void> {
      const task = tasks.get(taskId);
      if (!task) return;
      const s = store.get();
      const primary = primaryActionFor(task, {
        mayCommit: s.settings[taskId]?.mayCommit ?? false,
        queued: taskId in s.queued,
      });
      if (primary.intent.kind === "focus-terminal") focusTerminal();
      else if (primary.intent.kind === "dock-tab") openDockTab?.(primary.intent.tab);
      else if (primary.intent.kind === "action" && !primary.disabled) await act(taskId, primary.intent.action);
    },
    async rename(taskId: TaskId, title: string): Promise<void> {
      try {
        const task = await ipc.tasksRename(taskId, title);
        if (task && typeof task === "object") tasks.put(task);
      } catch (e) {
        fail(taskId, e);
      }
    },
    async stopAgent(taskId: TaskId): Promise<void> {
      const session = tasks.get(taskId)?.primary_session;
      if (!session) return;
      try {
        await ipc.sessionsStop(session);
      } catch (e) {
        fail(taskId, e);
      }
    },
    startFresh: (taskId: TaskId) => act(taskId, { type: "StartFresh" }),
    /** Plan 11 §6.3: inject the integrate prompt; a Precondition (no base yet, not a Done worktree task) shows here. */
    async integrate(taskId: TaskId): Promise<void> {
      try {
        await ipc.overlapIntegrate(taskId);
      } catch (e) {
        fail(taskId, e);
      }
    },
    /** Starts or resumes the primary session (idempotent); a new id that no task-changed brought is fetched. */
    async startSession(taskId: TaskId): Promise<void> {
      try {
        const session = await ipc.sessionsOpen(taskId);
        if (typeof session === "string" && tasks.get(taskId)?.primary_session !== session)
          tasks.put(await ipc.tasksGet(taskId));
      } catch (e) {
        fail(taskId, e);
      }
    },
    onCostUpdated,
    onTeamEvent(taskId: TaskId, event: TaskEvent): void {
      if (event.event.type === "StatusLine") put("contextPct", taskId, event.event.context_used_pct);
    },
    isOpen: (taskId: TaskId) => taskId in store.get().seen,
  };
}

export type TaskPageStore = ReturnType<typeof createTaskPageStore>;

export const taskPageStore = createTaskPageStore(
  live(
    "tasksGet",
    "tasksAction",
    "tasksRename",
    "sessionsStop",
    "sessionsOpen",
    "costSummary",
    "settingsGet",
    "overlapIntegrate",
  ),
);
