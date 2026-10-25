// Test and dev-playground fixtures: event builders that return the generated `TaskEvent` shape (so a wire-shape drift
// fails `npm run typecheck`), a six-step plan and an eighteen-event Build over it. Not imported by app code.
import type { JsonValue, TaskEvent } from "@/lib/bindings";
import { ORCHESTRATOR_ID } from "./events";
import type { CanvasStep, RepoRoot } from "./types";

export type JsonObject = { [key: string]: JsonValue };

export const MEMORY_DIR = "/mem/2026-10-24-canvas";
export const ROOTS: RepoRoot[] = [
  { root: "/repo/app", name: "app", color: "indigo" },
  { root: "/repo/api", name: "api", color: "cyan" },
];
export const TASK_ID = "t-0c4";

const BASE_MS = Date.parse("2026-10-24T05:00:00Z");

function ev(seq: number, agentId: string | null, event: TaskEvent["event"]): TaskEvent {
  return {
    seq,
    at: new Date(BASE_MS + seq * 1000).toISOString(),
    task_id: TASK_ID,
    agent_id: agentId === ORCHESTRATOR_ID ? null : agentId,
    event,
  };
}

export const fx = {
  sessionStart: (seq: number): TaskEvent =>
    ev(seq, null, { type: "SessionStart", session_id: "claude-1", transcript_path: "/t/claude-1.jsonl" }),
  prompt: (seq: number): TaskEvent => ev(seq, null, { type: "UserPromptSubmit" }),
  pre: (seq: number, agentId: string, tool: string, input: JsonObject): TaskEvent =>
    ev(seq, agentId, { type: "PreToolUse", tool, input }),
  post: (seq: number, agentId: string, tool: string, input: JsonObject, response: JsonValue = {}): TaskEvent =>
    ev(seq, agentId, { type: "PostToolUse", tool, input, response }),
  edit: (seq: number, agentId: string, path: string, oldText: string, newText: string): TaskEvent =>
    ev(seq, agentId, {
      type: "PostToolUse",
      tool: "Edit",
      input: { file_path: path, old_string: oldText, new_string: newText },
      response: {},
    }),
  agentStart: (seq: number, agentId: string, agentType: string, description?: string): TaskEvent =>
    ev(seq, null, {
      type: "SubagentStart",
      agent_id: agentId,
      agent_type: agentType,
      description: description ?? null,
    }),
  agentStop: (seq: number, agentId: string): TaskEvent => ev(seq, null, { type: "SubagentStop", agent_id: agentId }),
  stop: (seq: number, message: string, backgroundTasks = 0): TaskEvent =>
    ev(seq, null, { type: "Stop", last_assistant_message: message, background_tasks: backgroundTasks }),
  stopFailure: (seq: number, message: string): TaskEvent => ev(seq, null, { type: "StopFailure", message }),
  permission: (seq: number, tool: string): TaskEvent => ev(seq, null, { type: "PermissionRequest", tool }),
  notification: (seq: number, message: string): TaskEvent => ev(seq, null, { type: "Notification", message }),
  statusLine: (seq: number, costUsd: number): TaskEvent =>
    ev(seq, null, { type: "StatusLine", cost_usd: costUsd, input_tokens: 0, output_tokens: 0, context_used_pct: 0 }),
  unknown: (seq: number, name: string): TaskEvent => ev(seq, null, { type: "Unknown", name }),
  claimDenied: (seq: number, held: { path?: string; command?: string }, otherTask: string | null = null): TaskEvent =>
    ev(seq, null, {
      type: "ClaimDenied",
      repo: "/repo/app",
      path: held.path ?? null,
      command: held.command ?? null,
      other_task: otherTask,
    }),
  overlapDecided: (seq: number, path: string, otherTask: string | null, decision: string): TaskEvent =>
    ev(seq, null, { type: "OverlapDecided", repo: "/repo/app", path, other_task: otherTask, decision }),
  overlapNoted: (seq: number, path: string, otherTask: string, direction: "in" | "out"): TaskEvent =>
    ev(seq, null, { type: "OverlapNoted", repo: "/repo/app", path, other_task: otherTask, direction }),
};

const step = (number: number, title: string, files: string[], after: number[] = [], repo: string | null = null) => ({
  number,
  title,
  done: false,
  after,
  owners: [],
  files,
  repo,
});

export function samplePlan(): CanvasStep[] {
  return [
    step(1, "Scaffold", ["src/a.ts"]),
    step(2, "Toolbar", ["src/toolbar.tsx"]),
    step(3, "API route", ["src/route.ts"], [1], "/repo/api"),
    step(4, "Wire toolbar to API", ["src/toolbar.tsx", "src/a.ts"], [2, 3]),
    step(5, "Docs", ["README.md"], [4]),
    step(6, "Cleanup", []),
  ];
}

const S1 = "Step 1 · frontend: scaffold the feature";
const S3 = "Step 3 · backend: add the route";
const S24 = "Step 2, 4 · ui-polish: toolbar and wiring";
const PLAN = `${MEMORY_DIR}/plan.md`;

export function sampleBuild(): TaskEvent[] {
  return [
    fx.sessionStart(1),
    fx.prompt(2),
    fx.pre(3, "main", "Task", { description: S1 }),
    fx.agentStart(4, "a1", "frontend", S1),
    fx.edit(5, "a1", "/repo/app/src/a.ts", "", "line\nline"),
    fx.pre(6, "main", "Task", { description: S3 }),
    fx.agentStart(7, "a2", "backend", S3),
    fx.edit(8, "a2", "/repo/api/src/route.ts", "", "r1\nr2\nr3"),
    fx.edit(9, "a2", "/repo/app/src/a.ts", "line", "line changed"),
    fx.agentStop(10, "a1"),
    fx.edit(11, "main", PLAN, "- [ ] 1. Scaffold", "- [x] 1. Scaffold"),
    fx.pre(12, "a2", "Bash", { command: "npm test" }),
    fx.post(13, "a2", "Bash", { command: "npm test" }, { exit_code: 0 }),
    fx.agentStop(14, "a2"),
    fx.edit(15, "main", PLAN, "- [ ] 3. API route (after 1)", "- [x] 3. API route (after 1)"),
    fx.pre(16, "main", "Task", { description: S24 }),
    fx.agentStart(17, "a3", "general-purpose", S24),
    fx.edit(18, "a3", "/repo/app/src/toolbar.tsx", "", "t"),
  ];
}
