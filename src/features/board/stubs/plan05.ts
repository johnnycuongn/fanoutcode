// STUB(plan 05): `tasks_activity`, `cost_summary`, `git_changes` and the `team-event` event are not in
// `src/lib/bindings.ts` yet (Plan 05 adds them). This file is the board's thin typed wrapper over them, with the
// type names and fields of Plan 05's plan (overview A6, A19). When Plan 05 merges: re-export these types from
// `@/lib/bindings`, call `commands.tasksActivity` and
// `events.teamEvent.listen`, and delete the eslint-disable lines and the two Tauri imports.
// eslint-disable-next-line teamcode/no-manual-ipc -- STUB(plan 05): not in the generated bindings yet
import { invoke } from "@tauri-apps/api/core";
// eslint-disable-next-line teamcode/no-manual-ipc -- STUB(plan 05): not in the generated bindings yet
import { listen } from "@tauri-apps/api/event";
import type { AgentState, Container, JsonValue } from "@/lib/bindings";

export type TeamRow = { name: string; state: AgentState };
export type TaskActivity = {
  task_id: string;
  current_file: string | null;
  plus: number;
  minus: number;
  team: TeamRow[];
  cost_usd: number | null;
};

// `CostSummary`, `CostLine`, `LiveCost`, `RepoChanges` and `ChangedFile` (and the `cost_summary` / `git_changes` wrappers) are
// Plan 06's STUBs in `src/lib/pendingCommands.ts`; the board uses those rather than a second copy.
export type { ChangedFile, CostLine, CostSummary, LiveCost, RepoChanges } from "@/lib/pendingCommands";

export type HookEvent =
  | { type: "SessionStart"; session_id: string; transcript_path: string }
  | { type: "UserPromptSubmit" }
  | { type: "PreToolUse"; tool: string; input: JsonValue }
  | { type: "PostToolUse"; tool: string; input: JsonValue; response: JsonValue }
  | { type: "SubagentStart"; agent_id: string; agent_type: string; description: string | null }
  | { type: "SubagentStop"; agent_id: string }
  | { type: "PermissionRequest"; tool: string }
  | { type: "Notification"; message: string }
  | { type: "Stop"; last_assistant_message: string; background_tasks: number }
  | { type: "StopFailure"; message: string }
  | { type: "StatusLine"; cost_usd: number; input_tokens: number; output_tokens: number; context_used_pct: number }
  | { type: "Unknown"; name: string };

export type TaskEvent = { seq: number; at: string; task_id: string; agent_id: string | null; event: HookEvent };
export type TeamEventPayload = { task_id: string; event: TaskEvent };

async function call<T>(command: string, args: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (err) {
    throw err instanceof Error ? err : new Error(String(err));
  }
}

export const tasksActivity = (container: Container): Promise<TaskActivity[]> => call("tasks_activity", { container });

export async function listenTeamEvent(cb: (payload: TeamEventPayload) => void): Promise<() => void> {
  return listen<TeamEventPayload>("team-event", (e) => cb(e.payload));
}
