// Accessors over the generated `HookEvent` (internally tagged `type`, A1). With fixtures.ts these are the only places
// that spell the wire shape; every other model module reads events through them.
import type { TaskEvent } from "@/lib/bindings";
import type { StreamKind, ToolCall } from "./types";

export const ORCHESTRATOR_ID = "main";

export type HookKind =
  | "session-start"
  | "prompt"
  | "pre-tool"
  | "post-tool"
  | "agent-start"
  | "agent-stop"
  | "permission"
  | "notification"
  | "stop"
  | "stop-failure"
  | "status-line"
  | "overlap"
  | "unknown";

export function kindOf(e: TaskEvent): HookKind {
  switch (e.event.type) {
    case "SessionStart":
      return "session-start";
    case "UserPromptSubmit":
      return "prompt";
    case "PreToolUse":
      return "pre-tool";
    case "PostToolUse":
      return "post-tool";
    case "SubagentStart":
      return "agent-start";
    case "SubagentStop":
      return "agent-stop";
    case "PermissionRequest":
      return "permission";
    case "Notification":
      return "notification";
    case "Stop":
      return "stop";
    case "StopFailure":
      return "stop-failure";
    case "StatusLine":
      return "status-line";
    // Plan 11: the claim-check hook's denials and the app's own overlap lines
    case "ClaimDenied":
    case "ClaimNoted":
    case "OverlapDecided":
    case "OverlapNoted":
      return "overlap";
    case "Unknown":
      return "unknown";
  }
}

export function agentOf(e: TaskEvent): string {
  return e.agent_id ?? ORCHESTRATOR_ID;
}

type Obj = Record<string, unknown>;
function asObj(v: unknown): Obj {
  return v !== null && typeof v === "object" && !Array.isArray(v) ? (v as Obj) : {};
}

export function asPreTool(e: TaskEvent): { tool: string; input: Obj } | null {
  return e.event.type === "PreToolUse" ? { tool: e.event.tool, input: asObj(e.event.input) } : null;
}

export function asPostTool(e: TaskEvent): { tool: string; input: Obj; response: unknown } | null {
  return e.event.type === "PostToolUse"
    ? { tool: e.event.tool, input: asObj(e.event.input), response: e.event.response }
    : null;
}

export function asSubagentStart(
  e: TaskEvent,
): { agent_id: string; agent_type: string; description: string | null } | null {
  if (e.event.type !== "SubagentStart") return null;
  const { agent_id, agent_type, description } = e.event;
  return { agent_id, agent_type, description };
}

export function asSubagentStop(e: TaskEvent): { agent_id: string } | null {
  return e.event.type === "SubagentStop" ? { agent_id: e.event.agent_id } : null;
}

export function asStop(e: TaskEvent): { last_assistant_message: string; background_tasks: number } | null {
  if (e.event.type !== "Stop") return null;
  return { last_assistant_message: e.event.last_assistant_message, background_tasks: e.event.background_tasks };
}

export function asUnknown(e: TaskEvent): { name: string } | null {
  return e.event.type === "Unknown" ? { name: e.event.name } : null;
}

/** The wording of the hook rows (Outline, lenses): permission, notification, stop and failure payloads. */
export function asHookDetail(e: TaskEvent): { tool?: string; message?: string } {
  switch (e.event.type) {
    case "PermissionRequest":
      return { tool: e.event.tool };
    case "Notification":
    case "StopFailure":
      return { message: e.event.message };
    default:
      return {};
  }
}

/** An overlap line (Plan 11): a held edit or command, an edit made over another task's changes, a decision, or a note. */
export type OverlapDetail =
  | { kind: "held"; path: string | null; command: string | null; otherTask: string | null }
  | { kind: "overlapped"; path: string | null; otherTask: string | null }
  | { kind: "decided"; path: string; otherTask: string | null; decision: string }
  | { kind: "noted"; path: string; otherTask: string; direction: string };

export function asOverlap(e: TaskEvent): OverlapDetail | null {
  const v = e.event;
  switch (v.type) {
    case "ClaimDenied":
      return { kind: "held", path: v.path, command: v.command, otherTask: v.other_task };
    // "Ask before editing" off: the edit went ahead on top of the other task's changes
    case "ClaimNoted":
      return { kind: "overlapped", path: v.path, otherTask: v.other_task };
    case "OverlapDecided":
      return { kind: "decided", path: v.path, otherTask: v.other_task, decision: v.decision };
    case "OverlapNoted":
      return { kind: "noted", path: v.path, otherTask: v.other_task, direction: v.direction };
    default:
      return null;
  }
}

export const EDIT_TOOLS = ["Edit", "Write", "MultiEdit", "NotebookEdit"] as const;
/** The dispatch tool: `Task`, which newer Claude Code versions call `Agent` (Plan 05 accepts both). */
export const AGENT_TOOLS: readonly string[] = ["Task", "Agent"];

export function isEditTool(tool: string): boolean {
  return (EDIT_TOOLS as readonly string[]).includes(tool);
}

export function pathOf(tool: string, input: Obj): string | null {
  if (!isEditTool(tool)) return null;
  const v = input[tool === "NotebookEdit" ? "notebook_path" : "file_path"];
  return typeof v === "string" && v !== "" ? v : null;
}

export function streamKind(tool: string): StreamKind {
  if (isEditTool(tool)) return "edit";
  if (tool === "Bash") return "command";
  if (AGENT_TOOLS.includes(tool)) return "agent";
  return "read";
}

function str(v: unknown): string {
  return typeof v === "string" ? v : "";
}

export function toolTarget(tool: string, input: Obj): string {
  if (isEditTool(tool)) return pathOf(tool, input) ?? "";
  if (tool === "Read") return str(input.file_path);
  if (tool === "Bash") return str(input.command).slice(0, 60);
  if (tool === "Grep" || tool === "Glob") return str(input.pattern);
  if (AGENT_TOOLS.includes(tool)) return str(input.description);
  if (tool === "WebFetch") return str(input.url);
  return "";
}

/** A `PreToolUse` pairs with the next unpaired `PostToolUse` of the same agent and tool (FIFO). */
export function pairToolCalls(events: TaskEvent[]): ToolCall[] {
  const calls: ToolCall[] = [];
  const open = new Map<string, ToolCall[]>();
  for (const e of events) {
    const agentId = agentOf(e);
    const pre = asPreTool(e);
    if (pre) {
      const call: ToolCall = {
        preSeq: e.seq,
        postSeq: null,
        agentId,
        tool: pre.tool,
        input: pre.input,
        response: null,
        startedAt: e.at,
        durationMs: null,
        kind: streamKind(pre.tool),
        target: toolTarget(pre.tool, pre.input),
      };
      calls.push(call);
      const key = `${agentId}\u0000${pre.tool}`;
      const queue = open.get(key);
      if (queue) queue.push(call);
      else open.set(key, [call]);
      continue;
    }
    const post = asPostTool(e);
    if (!post) continue;
    const call = open.get(`${agentId}\u0000${post.tool}`)?.shift();
    if (call) {
      call.postSeq = e.seq;
      call.response = post.response;
      call.durationMs = Date.parse(e.at) - Date.parse(call.startedAt);
    } else {
      calls.push({
        preSeq: e.seq,
        postSeq: e.seq,
        agentId,
        tool: post.tool,
        input: post.input,
        response: post.response,
        startedAt: e.at,
        durationMs: null,
        kind: streamKind(post.tool),
        target: toolTarget(post.tool, post.input),
      });
    }
  }
  return calls;
}

/** Drops a repeated `seq` (the tailer replays the log after a relaunch), keeping the first, and sorts ascending. */
export function normalizeEvents(events: TaskEvent[]): TaskEvent[] {
  const seen = new Set<number>();
  const out: TaskEvent[] = [];
  for (const e of events) {
    if (seen.has(e.seq)) continue;
    seen.add(e.seq);
    out.push(e);
  }
  let sorted = true;
  for (let i = 1; i < out.length; i++) if (out[i].seq < out[i - 1].seq) sorted = false;
  return sorted ? out : out.sort((a, b) => a.seq - b.seq);
}
