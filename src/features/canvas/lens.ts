// The drill-in lenses (§10.7 "Drill-in"): By file (every file the target touched, who else did, its edits in order
// with a mini diff and the step served, then commands and hook firings) and Event stream (newest first, one row per
// tool call or hook, filterable by kind and agent).
import type { TaskEvent } from "@/lib/bindings";
import { pathOf } from "./events";
import { belongsTo, hookEntry, isHookRow, type OutlineEntry } from "./outline";
import type { CanvasModel, DrillTarget, EditRecord, StreamKind, ToolCall } from "./types";

export type DiffLine = { sign: "-" | "+"; text: string };
export type ByFileEdit = {
  seq: number;
  at: string;
  agentId: string;
  step: number | null;
  added: number;
  removed: number;
  diff: { lines: DiffLine[]; truncated: boolean };
};
export type ByFileGroup = {
  path: string;
  repo: string | null;
  owners: string[];
  added: number;
  removed: number;
  edits: ByFileEdit[];
};
export type RollUp = { commands: ToolCall[]; hooks: OutlineEntry[] };
export type StreamRow = {
  seq: number;
  at: string;
  kind: StreamKind;
  agentId: string;
  tool: string | null;
  target: string;
  result: "ok" | "error" | "pending" | null;
  durationMs: number | null;
  muted: boolean;
};
/** An empty set means "all". */
export type StreamFilter = { kinds: ReadonlySet<StreamKind>; agents: ReadonlySet<string> };

function split(text: string | null): string[] {
  return text === null || text === "" ? [] : text.split("\n");
}

/** Removed lines then added lines (no LCS), each side cut to `cap`. */
export function miniDiff(oldText: string | null, newText: string | null, cap = 12): ByFileEdit["diff"] {
  const removed = split(oldText);
  const added = split(newText);
  return {
    lines: [
      ...removed.slice(0, cap).map((text) => ({ sign: "-" as const, text })),
      ...added.slice(0, cap).map((text) => ({ sign: "+" as const, text })),
    ],
    truncated: removed.length > cap || added.length > cap,
  };
}

const agentIdOf = (id: string) => (id.startsWith("agent:") ? id.slice("agent:".length) : id);
const pathOfTarget = (id: string) => (id.startsWith("file:") ? id.slice("file:".length) : id);

/** The agents a target's lens covers: the agent itself, or every owner of the file. */
export function streamAgents(model: CanvasModel, target: DrillTarget): string[] {
  if (target.kind === "agent") return [agentIdOf(target.id)];
  return model.files.find((f) => f.path === pathOfTarget(target.id))?.owners ?? [];
}

function hooksOf(events: TaskEvent[], agents: string[]): OutlineEntry[] {
  const out: OutlineEntry[] = [];
  for (const e of events) {
    if (!isHookRow(e)) continue;
    const owner = agents.find((a) => belongsTo(e, a));
    if (owner !== undefined) out.push(hookEntry(e, owner));
  }
  return out;
}

function toByFileEdit(e: EditRecord): ByFileEdit {
  const { seq, at, agentId, step, added, removed } = e;
  return { seq, at, agentId, step, added, removed, diff: miniDiff(e.oldText, e.newText) };
}

export function byFileLens(
  model: CanvasModel,
  events: TaskEvent[],
  target: DrillTarget,
): { groups: ByFileGroup[]; rollUp: RollUp } {
  const agents = streamAgents(model, target);
  const agent = target.kind === "agent" ? agentIdOf(target.id) : null;
  const paths =
    agent !== null
      ? [...new Set(model.edits.filter((e) => e.agentId === agent).map((e) => e.path))]
      : [pathOfTarget(target.id)];
  const groups = paths.flatMap((path): ByFileGroup[] => {
    const file = model.files.find((f) => f.path === path);
    if (!file) return [];
    const edits = model.edits.filter((e) => e.path === path && (agent === null || e.agentId === agent));
    return [
      {
        path,
        repo: file.repo,
        owners: file.owners,
        added: file.added,
        removed: file.removed,
        edits: edits.map(toByFileEdit),
      },
    ];
  });
  const commands = model.toolCalls.filter((c) => c.tool === "Bash" && agents.includes(c.agentId));
  return { groups, rollUp: { commands, hooks: hooksOf(events, agents) } };
}

function resultOf(c: ToolCall): StreamRow["result"] {
  if (c.postSeq === null) return "pending";
  const r = c.response as { is_error?: unknown; exit_code?: unknown } | null;
  if (r && typeof r === "object" && (r.is_error === true || (typeof r.exit_code === "number" && r.exit_code !== 0)))
    return "error";
  return "ok";
}

export function eventStreamLens(
  model: CanvasModel,
  events: TaskEvent[],
  target: DrillTarget,
  filter: StreamFilter,
): StreamRow[] {
  const agents = streamAgents(model, target);
  const file = target.kind === "file" ? pathOfTarget(target.id) : null;
  const touches = (c: ToolCall) => file === null || (pathOf(c.tool, c.input) ?? c.input.file_path) === file;
  const rows: StreamRow[] = [
    ...model.toolCalls
      .filter((c) => agents.includes(c.agentId) && touches(c))
      .map((c): StreamRow => ({
        seq: c.preSeq,
        at: c.startedAt,
        kind: c.kind,
        agentId: c.agentId,
        tool: c.tool,
        target: c.target,
        result: resultOf(c),
        durationMs: c.durationMs,
        muted: false,
      })),
    ...hooksOf(events, agents).map((h): StreamRow => ({
      seq: h.seq,
      at: h.at,
      kind: "hook",
      agentId: h.agentId,
      tool: null,
      target: h.text,
      result: null,
      durationMs: null,
      muted: true,
    })),
  ];
  return rows
    .filter(
      (r) =>
        (filter.kinds.size === 0 || filter.kinds.has(r.kind)) &&
        (filter.agents.size === 0 || filter.agents.has(r.agentId)),
    )
    .sort((a, b) => b.seq - a.seq);
}
