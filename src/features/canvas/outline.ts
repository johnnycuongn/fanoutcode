// Outline mode (§10.7): plan.md as a live checklist whose active steps expand into their agents' tool and hook
// stream. `belongsTo` and `hookText` are shared with the drill-in lenses.
import type { TaskEvent } from "@/lib/bindings";
import { tasksStore } from "@/lib/stores/tasksStore";
import {
  agentOf,
  asHookDetail,
  asOverlap,
  asStop,
  asSubagentStart,
  asSubagentStop,
  kindOf,
  type OverlapDetail,
} from "./events";
import type { AgentNodeData, CanvasModel, StepNodeData, ToolCall } from "./types";

export type OutlineEntry = {
  seq: number;
  at: string;
  kind: "tool" | "hook";
  agentId: string;
  text: string;
  durationMs: number | null;
  muted: boolean;
};
export type OutlineItem = { step: StepNodeData; expanded: boolean; agents: string[]; stream: OutlineEntry[] };

const HOOK_KINDS = new Set([
  "agent-start",
  "agent-stop",
  "permission",
  "notification",
  "stop",
  "stop-failure",
  "overlap",
]);

/** The event is the agent's own, or the start or stop of that subagent. */
export function belongsTo(e: TaskEvent, agentId: string): boolean {
  return agentOf(e) === agentId || asSubagentStart(e)?.agent_id === agentId || asSubagentStop(e)?.agent_id === agentId;
}

export function isHookRow(e: TaskEvent): boolean {
  return HOOK_KINDS.has(kindOf(e));
}

/** The other task by its title when this window knows it, else by its id. */
function taskName(id: string): string {
  return tasksStore.get().byId[id]?.title ?? id;
}

function overlapText(o: OverlapDetail): string {
  const other = o.otherTask === null ? "" : ` · ${taskName(o.otherTask)}`;
  switch (o.kind) {
    case "held":
      return o.path !== null ? `edit held: ${o.path}${other}` : `command held: ${o.command ?? ""}${other}`;
    case "overlapped":
      return `edited over: ${o.path ?? ""}${other}`;
    case "ran":
      return `command ran: ${o.command}${other}`;
    case "decided":
      return `decided ${o.decision}: ${o.path}${other}`;
    case "noted":
      return o.direction === "in"
        ? `note from ${taskName(o.otherTask)}: ${o.path}`
        : `note sent to ${taskName(o.otherTask)}: ${o.path}`;
  }
}

export function hookText(e: TaskEvent): string {
  const overlap = asOverlap(e);
  if (overlap) return overlapText(overlap);
  const start = asSubagentStart(e);
  if (start) return `started: ${start.description ?? start.agent_type}`;
  if (asSubagentStop(e)) return "stopped";
  const stop = asStop(e);
  if (stop) return stop.last_assistant_message.split("\n")[0] ?? "";
  const detail = asHookDetail(e);
  switch (kindOf(e)) {
    case "permission":
      return `permission: ${detail.tool ?? ""}`;
    case "stop-failure":
      return `error: ${detail.message ?? ""}`;
    default:
      return detail.message ?? "";
  }
}

export function toolEntry(c: ToolCall): OutlineEntry {
  const text = c.target ? `${c.tool} ${c.target}` : c.tool;
  return {
    seq: c.preSeq,
    at: c.startedAt,
    kind: "tool",
    agentId: c.agentId,
    text,
    durationMs: c.durationMs,
    muted: false,
  };
}

export function hookEntry(e: TaskEvent, agentId: string): OutlineEntry {
  return { seq: e.seq, at: e.at, kind: "hook", agentId, text: hookText(e), durationMs: null, muted: true };
}

/** The tool calls and hook rows of `agents`, each inside that agent's own start…stop window, ascending. */
export function streamOf(model: CanvasModel, events: TaskEvent[], agents: AgentNodeData[]): OutlineEntry[] {
  const out: OutlineEntry[] = [];
  for (const a of agents) {
    const inside = (seq: number) => seq >= a.startedSeq && seq <= (a.stoppedSeq ?? Infinity);
    for (const c of model.toolCalls) if (c.agentId === a.agentId && inside(c.preSeq)) out.push(toolEntry(c));
    for (const e of events)
      if (isHookRow(e) && belongsTo(e, a.agentId) && inside(e.seq)) out.push(hookEntry(e, a.agentId));
  }
  return out.sort((x, y) => x.seq - y.seq);
}

export function buildOutline(model: CanvasModel, events: TaskEvent[]): OutlineItem[] {
  const byId = new Map(model.agents.map((a) => [a.agentId, a]));
  return model.steps.map((step) => {
    const ids: string[] = [];
    const add = (id: string) => {
      if (!ids.includes(id)) ids.push(id);
    };
    for (const a of model.agents) if (a.steps.includes(step.number)) add(a.agentId);
    for (const e of model.edits) if (e.step === step.number) add(e.agentId);
    const expanded = step.status === "active";
    const agents = ids.map((id) => byId.get(id)).filter((a): a is AgentNodeData => a !== undefined);
    return { step, expanded, agents: ids, stream: expanded ? streamOf(model, events, agents) : [] };
  });
}
