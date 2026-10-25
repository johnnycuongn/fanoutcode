// Step cards: status from the ticks in plan.md and the dispatches, owners, files, the live tool line, progress and
// cost (§10.7 "Nodes").
import type { TaskEvent } from "@/lib/bindings";
import type { AgentRecord } from "./attribution";
import { asPostTool, isEditTool, pathOf } from "./events";
import { isMemoryFile } from "./files";
import type { AgentCost, CanvasStep, EditRecord, FileNodeData, StepNodeData, ToolCall } from "./types";

const TICK = /^\s*[-*]\s*\[[xX]\]\s*(\d+)\./gm;

function ticks(text: unknown, into: Set<number>): void {
  if (typeof text !== "string") return;
  for (const m of text.matchAll(TICK)) into.add(Number(m[1]));
}

/** Steps ticked `- [x] N.` by an edit of the task's own `plan.md` among `events`. */
export function doneStepsAt(events: TaskEvent[], memoryDir: string): Set<number> {
  const done = new Set<number>();
  for (const e of events) {
    const post = asPostTool(e);
    if (!post || !isEditTool(post.tool)) continue;
    const path = pathOf(post.tool, post.input);
    if (path === null || !path.endsWith("plan.md") || !isMemoryFile(path, memoryDir)) continue;
    ticks(post.input.new_string, done);
    ticks(post.input.content, done);
    if (Array.isArray(post.input.edits)) {
      for (const edit of post.input.edits as Record<string, unknown>[]) ticks(edit?.new_string, done);
    }
  }
  return done;
}

/** An agent's cost split evenly across the steps it was dispatched for; null when unknown or it has none. */
export function stepCost(agent: AgentRecord, costs: AgentCost[]): number | null {
  if (agent.steps.length === 0) return null;
  const c = costs.find((x) => x.agentId === agent.agentId);
  return c ? c.costUsd / agent.steps.length : null;
}

function pushTo<K, V>(map: Map<K, V[]>, key: K, value: V): void {
  const list = map.get(key);
  if (list) {
    if (!list.includes(value)) list.push(value);
  } else map.set(key, [value]);
}

export function deriveSteps(args: {
  planSteps: CanvasStep[];
  agents: AgentRecord[];
  edits: EditRecord[];
  files: FileNodeData[];
  toolCalls: ToolCall[];
  done: Set<number>;
  costs: AgentCost[];
}): StepNodeData[] {
  const { planSteps, agents, edits, files, toolCalls, done, costs } = args;
  const listing = new Map<number, AgentRecord[]>();
  for (const a of agents) for (const n of a.steps) pushTo(listing, n, a);
  const editAgents = new Map<number, string[]>();
  const editedPaths = new Map<number, string[]>();
  for (const e of edits) {
    if (e.step === null) continue;
    pushTo(editAgents, e.step, e.agentId);
    pushTo(editedPaths, e.step, e.path);
  }
  const latest = new Map<string, ToolCall>();
  for (const c of toolCalls) {
    const prev = latest.get(c.agentId);
    if (!prev || c.preSeq > prev.preSeq) latest.set(c.agentId, c);
  }

  return planSteps.map((p) => {
    const n = p.number;
    const mine = listing.get(n) ?? [];
    const status = p.done || done.has(n) ? "done" : mine.length > 0 || editAgents.has(n) ? "active" : "upcoming";
    const owners: string[] = [];
    for (const r of [...mine.map((a) => a.role), ...p.owners]) if (!owners.includes(r)) owners.push(r);

    let call: ToolCall | null = null;
    for (const id of [...mine.map((a) => a.agentId), ...(editAgents.get(n) ?? [])]) {
      const c = latest.get(id);
      if (c && (!call || c.preSeq > call.preSeq)) call = c;
    }

    const touchedPaths = editedPaths.get(n) ?? [];
    const touched = p.files.filter((f) => touchedPaths.some((path) => path === f || path.endsWith("/" + f))).length;
    const progress = status === "done" ? 1 : p.files.length === 0 ? null : touched / p.files.length;

    const shares = mine.map((a) => stepCost(a, costs)).filter((c): c is number => c !== null);
    return {
      number: n,
      title: p.title,
      status,
      working: mine.some((a) => a.working),
      owners,
      after: p.after,
      repo: p.repo,
      files: files
        .filter((f) => f.steps.includes(n))
        .map((f) => ({ path: f.path, added: f.added, removed: f.removed, shared: f.owners.length >= 2 })),
      toolLine: call ? `${call.tool} ${call.target}` : null,
      progress,
      costUsd: shares.length > 0 ? shares.reduce((a, b) => a + b, 0) : null,
      stackIndex: null,
    };
  });
}
