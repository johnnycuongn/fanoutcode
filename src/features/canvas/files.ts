// Edits and file records from the edit tools' `PostToolUse` events (§10.7 "Data"): who edited what, by how much,
// for which plan step, and when a file became shared.
import type { TaskEvent } from "@/lib/bindings";
import type { AgentRecord } from "./attribution";
import { agentOf, asPostTool, isEditTool, pathOf } from "./events";
import type { CanvasStep, EditRecord, FileNodeData, FileStat } from "./types";

function lines(text: unknown): number {
  return typeof text !== "string" || text === "" ? 0 : text.split("\n").length;
}

export function editDelta(tool: string, input: Record<string, unknown>): { added: number; removed: number } {
  switch (tool) {
    case "Edit":
      return { added: lines(input.new_string), removed: lines(input.old_string) };
    case "MultiEdit": {
      const edits = Array.isArray(input.edits) ? (input.edits as Record<string, unknown>[]) : [];
      return edits.reduce<{ added: number; removed: number }>(
        (sum, e) => ({ added: sum.added + lines(e?.new_string), removed: sum.removed + lines(e?.old_string) }),
        { added: 0, removed: 0 },
      );
    }
    case "Write":
      return { added: lines(input.content), removed: 0 };
    case "NotebookEdit":
      return { added: lines(input.new_source), removed: 0 };
    default:
      return { added: 0, removed: 0 };
  }
}

export function isMemoryFile(path: string, memoryDir: string): boolean {
  const dir = memoryDir.replace(/\/+$/, "");
  return dir !== "" && (path === dir || path.startsWith(dir + "/"));
}

function lists(step: CanvasStep | undefined, path: string): boolean {
  return step !== undefined && step.files.some((f) => path === f || path.endsWith("/" + f));
}

/** The plan step an edit served: a dispatched step listing the file, the only dispatched step, the first open one. */
export function attributeStep(agentSteps: number[], path: string, planSteps: CanvasStep[]): number | null {
  const byNumber = new Map(planSteps.map((s) => [s.number, s]));
  const candidates = agentSteps.length > 0 ? agentSteps : planSteps.map((s) => s.number);
  const listed = candidates.find((n) => lists(byNumber.get(n), path));
  if (listed !== undefined) return listed;
  if (agentSteps.length === 1) return agentSteps[0];
  if (agentSteps.length > 1) return agentSteps.find((n) => !byNumber.get(n)?.done) ?? null;
  return null;
}

export function deriveEdits(
  events: TaskEvent[],
  memoryDir: string,
  agents: AgentRecord[],
  planSteps: CanvasStep[],
): EditRecord[] {
  const stepsOf = new Map(agents.map((a) => [a.agentId, a.steps]));
  const out: EditRecord[] = [];
  for (const e of events) {
    const post = asPostTool(e);
    if (!post || !isEditTool(post.tool)) continue;
    const path = pathOf(post.tool, post.input);
    if (path === null || isMemoryFile(path, memoryDir)) continue;
    const agentId = agentOf(e);
    const isEdit = post.tool === "Edit";
    out.push({
      seq: e.seq,
      at: e.at,
      agentId,
      tool: post.tool,
      path,
      ...editDelta(post.tool, post.input),
      step: attributeStep(stepsOf.get(agentId) ?? [], path, planSteps),
      oldText: isEdit && typeof post.input.old_string === "string" ? post.input.old_string : null,
      newText: isEdit && typeof post.input.new_string === "string" ? post.input.new_string : null,
    });
  }
  return out;
}

/** One record per edited path; `fileStats` (git) win for net `+/-` and add owners git attributed. */
export function deriveFiles(edits: EditRecord[], fileStats: FileStat[] = []): FileNodeData[] {
  const byPath = new Map<string, FileNodeData>();
  for (const e of edits) {
    let f = byPath.get(e.path);
    if (!f) {
      f = { path: e.path, repo: null, added: 0, removed: 0, owners: [], sharedAt: null, steps: [] };
      byPath.set(e.path, f);
    }
    f.added += e.added;
    f.removed += e.removed;
    if (!f.owners.includes(e.agentId)) {
      f.owners.push(e.agentId);
      if (f.owners.length === 2) f.sharedAt = e.seq;
    }
    if (e.step !== null && !f.steps.includes(e.step)) f.steps.push(e.step);
  }
  for (const s of fileStats) {
    const f = byPath.get(s.path);
    if (!f) continue;
    f.added = s.added;
    f.removed = s.removed;
    for (const o of s.owners) if (!f.owners.includes(o)) f.owners.push(o);
  }
  return [...byPath.values()];
}
