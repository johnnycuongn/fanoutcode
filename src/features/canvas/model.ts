// The canvas model: a pure function from plan steps and the task's hook events to agents, steps, files, edits and
// tool calls (§10.7 "Data"). No DOM, no IPC; linear in the number of events.
import { deriveAgents } from "./attribution";
import { normalizeEvents, pairToolCalls } from "./events";
import { deriveEdits, deriveFiles } from "./files";
import { nodeId } from "./graph";
import { repoFor } from "./repo";
import { deriveSteps, doneStepsAt } from "./steps";
import type { AgentNodeData, CanvasInput, CanvasModel } from "./types";

export { toGraph, layoutKey, nodeId } from "./graph";

export function deriveCanvas(input: CanvasInput): CanvasModel {
  const { steps: planSteps, memoryDir, roots, fileStats = [], costs = [] } = input;
  const events = normalizeEvents(input.events);
  const records = deriveAgents(events);
  const edits = deriveEdits(events, memoryDir, records, planSteps);
  const files = deriveFiles(edits, fileStats).map((f) => ({ ...f, repo: repoFor(f.path, roots)?.root ?? null }));
  const toolCalls = pairToolCalls(events);
  const home = roots[0]?.root ?? null;
  const steps = deriveSteps({
    planSteps,
    agents: records,
    edits,
    files,
    toolCalls,
    done: doneStepsAt(events, memoryDir),
    costs,
  }).map((s) => ({ ...s, repo: s.repo ?? home }));

  const owned = new Map<string, string[]>();
  for (const f of files) {
    for (const o of f.owners) {
      const list = owned.get(o);
      if (list) list.push(nodeId.file(f.path));
      else owned.set(o, [nodeId.file(f.path)]);
    }
  }
  const costOf = new Map(costs.map((c) => [c.agentId, c.costUsd]));
  const agents: AgentNodeData[] = records.map((r) => ({
    ...r,
    files: owned.get(r.agentId) ?? [],
    costUsd: costOf.get(r.agentId) ?? null,
  }));
  return { agents, steps, files, edits, toolCalls, lastSeq: events.at(-1)?.seq ?? 0 };
}
