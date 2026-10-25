// The canvas store's state core: one entry per open task, derived (pure model, then graph) whenever an input changes.
// store.ts loads and keeps entries live; layoutRunner.ts sets their positions.
import type { Project, ResolvedSettings, Stage, Task, TaskEvent } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { scheduleLayout } from "./layoutRunner";
import { deriveCanvas, layoutKey, toGraph } from "./model";
import { eventsUntil } from "./replay";
import type {
  AgentCost,
  CanvasGraph,
  CanvasModel,
  CanvasPosition,
  CanvasStep,
  DrillTarget,
  FileStat,
  RepoRoot,
  TeamViewMode,
} from "./types";

export type CanvasSettings = {
  detail: ResolvedSettings["detail_level"];
  showCost: boolean;
  arrangement: ResolvedSettings["team_view_arrangement"];
  layout: ResolvedSettings["team_canvas_layout"];
  lens: ResolvedSettings["team_drill_in_lens"];
  placement: ResolvedSettings["team_view_placement"];
};

export type CanvasState = {
  taskId: string;
  task: Task | null;
  roots: RepoRoot[];
  events: TaskEvent[];
  steps: CanvasStep[];
  fileStats: FileStat[];
  costs: AgentCost[];
  settings: CanvasSettings;
  viewMode: TeamViewMode;
  selection: DrillTarget | null;
  replaySeq: number | null;
  positions: CanvasPosition[];
  savedIds: ReadonlySet<string>;
  model: CanvasModel;
  graph: CanvasGraph;
  prevStage: Stage | null;
  loading: boolean;
};

export const DEFAULT_CANVAS_SETTINGS: CanvasSettings = {
  detail: "Medium",
  showCost: false,
  arrangement: "HubGraph",
  layout: "AutoReflow",
  lens: "ByFile",
  placement: "DockGrows",
};

export function toSettings(r: ResolvedSettings): CanvasSettings {
  return {
    detail: r.detail_level,
    showCost: r.cost_overview,
    arrangement: r.team_view_arrangement,
    layout: r.team_canvas_layout,
    lens: r.team_drill_in_lens,
    placement: r.team_view_placement,
  };
}

function derive(s: CanvasState): Pick<CanvasState, "model" | "graph"> {
  const visible = s.replaySeq === null ? s.events : eventsUntil(s.events, s.replaySeq);
  const model = deriveCanvas({
    steps: s.steps,
    events: visible,
    memoryDir: s.task?.memory_dir ?? "",
    roots: s.roots,
    fileStats: s.fileStats,
    costs: s.costs,
  });
  return { model, graph: toGraph(model, s.settings.arrangement) };
}

export function blank(taskId: string): CanvasState {
  const s: Omit<CanvasState, "model" | "graph"> = {
    taskId,
    task: null,
    roots: [],
    events: [],
    steps: [],
    fileStats: [],
    costs: [],
    settings: DEFAULT_CANVAS_SETTINGS,
    viewMode: "canvas",
    selection: null,
    replaySeq: null,
    positions: [],
    savedIds: new Set(),
    prevStage: null,
    loading: true,
  };
  return { ...s, ...derive(s as CanvasState) };
}

export const store = createStore<{ byTask: Record<string, CanvasState> }>({ byTask: {} });
export type Entry = {
  refs: number;
  ready: Promise<void>;
  unlisten: (() => void)[];
  projects: Project[];
  key: string | null;
};
export const entries = new Map<string, Entry>();
const blanks = new Map<string, CanvasState>();

function blankFor(taskId: string): CanvasState {
  let b = blanks.get(taskId);
  if (!b) blanks.set(taskId, (b = blank(taskId)));
  return b;
}

export function getCanvas(taskId: string): CanvasState {
  return store.get().byTask[taskId] ?? blankFor(taskId);
}

export function useCanvas(taskId: string): CanvasState {
  return store.useSelector((s) => s.byTask[taskId] ?? blankFor(taskId));
}

export function isCanvasOpen(taskId: string): boolean {
  return entries.has(taskId);
}

export function put(taskId: string, next: CanvasState): void {
  store.set((s) => ({ byTask: { ...s.byTask, [taskId]: next } }));
}

const DERIVED_FROM = ["events", "steps", "fileStats", "costs", "replaySeq", "settings", "roots", "task"] as const;

/** Merges a patch; re-derives when an input changed, and schedules a re-layout when the layout key moved. */
export function update(taskId: string, patch: Partial<CanvasState>): void {
  const entry = entries.get(taskId);
  if (!entry) return;
  const prev = getCanvas(taskId);
  let next = { ...prev, ...patch };
  if (DERIVED_FROM.some((k) => k in patch && patch[k] !== prev[k])) next = { ...next, ...derive(next) };
  put(taskId, next);
  const key = layoutKey(next.graph);
  if (entry.key !== null && key !== entry.key) scheduleLayout(taskId, "change");
  if (entry.key !== null || !next.loading) entry.key = key;
}

/** Positions from the layout runner or a drag; no re-derivation. */
export function setPositions(taskId: string, positions: CanvasPosition[]): void {
  if (entries.has(taskId)) put(taskId, { ...getCanvas(taskId), positions });
}

export function markSaved(taskId: string, ids: string[]): void {
  if (entries.has(taskId)) put(taskId, { ...getCanvas(taskId), savedIds: new Set(ids) });
}
