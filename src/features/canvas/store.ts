// The canvas module store: loads a task's canvas from hooks_list, hooks_plan_steps, git_changes, cost_summary and
// settings_get, keeps it live through team-event, task-changed, git-changes and cost-updated, and holds the view
// actions. State and derivation live in canvasState.ts; debounced layout and persistence in layoutRunner.ts.
import { events as ipcEvents } from "@/lib/bindings";
import type { Project, Task, TaskEvent } from "@/lib/bindings";
import { live } from "@/lib/ipcResult";
import { fromCostSummary, fromPlanDoc, fromRepoChanges, rootsFor, scopeFor } from "./adapters";
import {
  DEFAULT_CANVAS_SETTINGS,
  blank,
  entries,
  getCanvas,
  put,
  setPositions,
  store,
  toSettings,
  update,
  type CanvasSettings,
  type Entry,
} from "./canvasState";
import { asPostTool, isEditTool, normalizeEvents, pathOf } from "./events";
import { isMemoryFile } from "./files";
import { cancelLayout, flushPersist, runLayout, scheduleLayout, schedulePersist } from "./layoutRunner";
import { layoutTrigger } from "./positions";
import { replayBounds } from "./replay";
import type { CanvasPosition, CanvasStep, DrillTarget, RepoRoot, TeamViewMode } from "./types";

export { DEFAULT_CANVAS_SETTINGS, getCanvas, isCanvasOpen, useCanvas } from "./canvasState";
export type { CanvasSettings, CanvasState } from "./canvasState";

const ipc = live(
  "tasksGet",
  "projectsList",
  "settingsGet",
  "hooksList",
  "hooksPlanSteps",
  "gitChanges",
  "costSummary",
  "canvasPositionsGet",
);

function touchesPlan(e: TaskEvent, memoryDir: string | undefined): boolean {
  const post = asPostTool(e);
  const path = post && isEditTool(post.tool) ? pathOf(post.tool, post.input) : null;
  return path !== null && memoryDir !== undefined && path.endsWith("plan.md") && isMemoryFile(path, memoryDir);
}

async function refetchPlan(taskId: string): Promise<void> {
  const doc = await ipc.hooksPlanSteps(taskId).catch(() => null);
  if (doc) update(taskId, { steps: fromPlanDoc(doc, getCanvas(taskId).roots) });
}

/** The `team-event` path: one event appended (a replayed `seq` is dropped), plan.md edits re-read the steps. */
export function pushEvent(taskId: string, event: TaskEvent): void {
  const s = getCanvas(taskId);
  const last = s.events.at(-1)?.seq ?? -Infinity;
  const events = event.seq > last ? [...s.events, event] : normalizeEvents([...s.events, event]);
  if (events.length === s.events.length) return;
  update(taskId, { events });
  if (touchesPlan(event, s.task?.memory_dir)) void refetchPlan(taskId);
}

async function onTaskChanged(taskId: string, task: Task): Promise<void> {
  const entry = entries.get(taskId);
  if (!entry) return;
  const prevStage = getCanvas(taskId).task?.stage ?? null;
  update(taskId, { task, roots: rootsFor(task, entry.projects), prevStage });
  if (layoutTrigger(prevStage, task.stage, true) === "build-start") scheduleLayout(taskId, "build-start");
  const settings = await ipc.settingsGet(scopeFor(task)).catch(() => null);
  if (!settings) return;
  const before = getCanvas(taskId).settings;
  const next = toSettings(settings);
  update(taskId, { settings: next });
  if (next.arrangement !== before.arrangement || next.layout !== before.layout) scheduleLayout(taskId, "tidy");
}

async function subscribe(taskId: string): Promise<(() => void)[]> {
  return Promise.all([
    ipcEvents.teamEvent.listen((e) => {
      if (e.payload.task_id === taskId) pushEvent(taskId, e.payload.event);
    }),
    ipcEvents.taskChanged.listen((e) => {
      if (e.payload.task.id === taskId) void onTaskChanged(taskId, e.payload.task);
    }),
    ipcEvents.gitChanges.listen((e) => {
      if (e.payload.task_id !== taskId) return;
      void ipc.gitChanges(taskId).then(
        (c) => update(taskId, { fileStats: fromRepoChanges(c) }),
        () => undefined,
      );
    }),
    ipcEvents.costUpdated.listen((e) => {
      const task = getCanvas(taskId).task;
      if (e.payload.task_id !== taskId || !task) return;
      void ipc.costSummary(scopeFor(task)).then(
        (c) => update(taskId, { costs: fromCostSummary(c) }),
        () => undefined,
      );
    }),
  ]);
}

async function load(taskId: string, entry: Entry): Promise<void> {
  const task = await ipc.tasksGet(taskId).catch(() => null);
  if (entries.get(taskId) !== entry) return;
  if (!task) return update(taskId, { loading: false });
  const scope = scopeFor(task);
  const [projects, settings, events, plan, changes, cost, positions] = await Promise.all([
    ipc.projectsList().catch(() => [] as Project[]),
    ipc.settingsGet(scope).catch(() => null),
    ipc.hooksList(taskId).catch(() => [] as TaskEvent[]),
    ipc.hooksPlanSteps(taskId).catch(() => null),
    ipc.gitChanges(taskId).catch(() => []),
    ipc.costSummary(scope).catch(() => null),
    ipc.canvasPositionsGet(taskId).catch(() => [] as CanvasPosition[]),
  ]);
  if (entries.get(taskId) !== entry) return;
  entry.projects = projects;
  const roots = rootsFor(task, projects);
  update(taskId, {
    task,
    roots,
    settings: settings ? toSettings(settings) : DEFAULT_CANVAS_SETTINGS,
    events: normalizeEvents(events),
    steps: plan ? fromPlanDoc(plan, roots) : [],
    fileStats: fromRepoChanges(changes),
    costs: cost ? fromCostSummary(cost) : [],
    positions,
    savedIds: new Set(positions.map((p) => p.node_id)),
    prevStage: task.stage,
    loading: false,
  });
  const unlisten = await subscribe(taskId);
  if (entries.get(taskId) !== entry) return unlisten.forEach((u) => u());
  entry.unlisten = unlisten;
  scheduleLayout(taskId, layoutTrigger(null, task.stage, positions.length > 0));
}

/** Loads the task's canvas and keeps it live; reference-counted, so the dock and a pane can both show it. */
export function openCanvas(taskId: string): Promise<void> {
  const open = entries.get(taskId);
  if (open) {
    open.refs++;
    return open.ready;
  }
  const entry: Entry = { refs: 1, ready: Promise.resolve(), unlisten: [], projects: [], key: null };
  entries.set(taskId, entry);
  put(taskId, blank(taskId));
  entry.ready = load(taskId, entry);
  return entry.ready;
}

export function closeCanvas(taskId: string): void {
  const entry = entries.get(taskId);
  if (!entry || --entry.refs > 0) return;
  flushPersist(taskId);
  cancelLayout(taskId);
  entry.unlisten.forEach((u) => u());
  entries.delete(taskId);
  store.set((s) => {
    const byTask = { ...s.byTask };
    delete byTask[taskId];
    return { byTask };
  });
}

export function setViewMode(taskId: string, viewMode: TeamViewMode): void {
  update(taskId, { viewMode });
}

export function select(taskId: string, selection: DrillTarget | null): void {
  update(taskId, { selection });
}

export function setReplaySeq(taskId: string, replaySeq: number | null): void {
  update(taskId, { replaySeq });
}

/** Replay from the end of the log; `stopReplay` returns to live. */
export function startReplay(taskId: string): void {
  const bounds = replayBounds(getCanvas(taskId).events);
  if (bounds) setReplaySeq(taskId, bounds.lastSeq);
}

export function stopReplay(taskId: string): void {
  setReplaySeq(taskId, null);
}

/** A drag ended: that node's position, persisted 500 ms after the last move. */
export function moveNode(taskId: string, nodeId: string, x: number, y: number): void {
  const s = getCanvas(taskId);
  const positions = s.positions.some((p) => p.node_id === nodeId)
    ? s.positions.map((p) => (p.node_id === nodeId ? { node_id: nodeId, x, y } : p))
    : [...s.positions, { node_id: nodeId, x, y }];
  setPositions(taskId, positions);
  schedulePersist(taskId);
}

export function tidy(taskId: string): Promise<void> {
  cancelLayout(taskId);
  return runLayout(taskId, "tidy");
}

/** Dev playground and tests only: an open entry with a task and plan but no IPC. */
export function seedCanvas(
  taskId: string,
  seed: { task: Task; steps: CanvasStep[]; roots: RepoRoot[]; settings?: Partial<CanvasSettings> },
): void {
  if (!entries.has(taskId))
    entries.set(taskId, { refs: 1, ready: Promise.resolve(), unlisten: [], projects: [], key: null });
  put(taskId, blank(taskId));
  update(taskId, {
    task: seed.task,
    steps: seed.steps,
    roots: seed.roots,
    settings: { ...DEFAULT_CANVAS_SETTINGS, ...seed.settings },
    loading: false,
  });
  scheduleLayout(taskId, "initial");
}

export function updateSettings(taskId: string, settings: Partial<CanvasSettings>): void {
  const before = getCanvas(taskId).settings;
  update(taskId, { settings: { ...before, ...settings } });
  scheduleLayout(taskId, "tidy");
}
