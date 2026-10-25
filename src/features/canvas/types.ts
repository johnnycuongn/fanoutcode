// The canvas's own types (webview-only; nothing here crosses IPC). The model modules turn plan steps and the task's
// hook events into these; the components render them. Wire shapes come from src/lib/bindings.ts only.
import type { DetailLevel, TaskEvent } from "@/lib/bindings";

export type StepStatus = "upcoming" | "active" | "done";
export type TeamViewMode = "canvas" | "outline";
/** `id` is the node id (`agent:<id>` is accepted as the bare agent id too). */
export type DrillTarget = { kind: "agent"; id: string } | { kind: "file"; id: string };
export type RepoRoot = { root: string; name: string; color: string };
export type FileStat = { path: string; added: number; removed: number; owners: string[] };
export type AgentCost = { agentId: string; costUsd: number };

/**
 * One `plan.md` step as the canvas reads it. Built from Plan 05's `PlanStep` and `PlanDoc.team` by
 * `adapters.fromPlanDoc`: `after` is the explicit `(after …)` list only, `owners` the Team roles naming the step,
 * `files` the files the plan lists for it (none today), `repo` the step's root path or null for the home root.
 */
export type CanvasStep = {
  number: number;
  title: string;
  done: boolean;
  after: number[];
  owners: string[];
  files: string[];
  repo: string | null;
};

export type AgentNodeData = {
  agentId: string;
  role: string;
  adhoc: boolean;
  working: boolean;
  steps: number[];
  description: string | null;
  startedSeq: number;
  stoppedSeq: number | null;
  files: string[];
  costUsd: number | null;
};
export type StepFile = { path: string; added: number; removed: number; shared: boolean };
export type StepNodeData = {
  number: number;
  title: string;
  status: StepStatus;
  working: boolean;
  owners: string[];
  after: number[];
  repo: string | null;
  files: StepFile[];
  toolLine: string | null;
  progress: number | null;
  costUsd: number | null;
  stackIndex: number | null;
};
export type FileNodeData = {
  path: string;
  repo: string | null;
  added: number;
  removed: number;
  owners: string[];
  sharedAt: number | null;
  steps: number[];
};
export type CanvasNode =
  | { id: string; kind: "agent"; data: AgentNodeData }
  | { id: string; kind: "step"; data: StepNodeData }
  | { id: string; kind: "file"; data: FileNodeData };
export type CanvasEdge = { id: string; source: string; target: string; kind: "touches" | "depends"; working: boolean };
export type CanvasGraph = { nodes: CanvasNode[]; edges: CanvasEdge[] };
export type EditRecord = {
  seq: number;
  at: string;
  agentId: string;
  tool: string;
  path: string;
  added: number;
  removed: number;
  step: number | null;
  oldText: string | null;
  newText: string | null;
};
export type StreamKind = "edit" | "command" | "read" | "agent" | "hook";
export type ToolCall = {
  preSeq: number;
  postSeq: number | null;
  agentId: string;
  tool: string;
  input: Record<string, unknown>;
  response: unknown | null;
  startedAt: string;
  durationMs: number | null;
  kind: StreamKind;
  target: string;
};
export type CanvasInput = {
  steps: CanvasStep[];
  events: TaskEvent[];
  memoryDir: string;
  roots: RepoRoot[];
  fileStats?: FileStat[];
  costs?: AgentCost[];
};
export type CanvasModel = {
  agents: AgentNodeData[];
  steps: StepNodeData[];
  files: FileNodeData[];
  edits: EditRecord[];
  toolCalls: ToolCall[];
  lastSeq: number;
};
export type ViewOptions = {
  detail: DetailLevel;
  showCost: boolean;
  showRepo: boolean;
  roots: RepoRoot[];
  reduced: boolean;
};
