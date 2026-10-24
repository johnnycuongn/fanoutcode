// Text and token helpers shared by the shell, the board and the task page. `chipText` is the TypeScript mirror of the
// Rust `handback::chip_text` (A2); format.test.ts checks it row for row against src/test/fixtures/handback.json, which
// a Plan 03 Rust test writes.
import type { AgentStatus, StageToken } from "@/components/ui";
import type { AgentState, HandbackReason, Stage, Task } from "@/lib/bindings";

const STAGE_DONE: Record<Stage, string> = {
  Spec: "Spec ready",
  Plan: "Plan written, go?",
  Build: "Build done",
  Review: "Review written",
  Test: "Tests passed",
  Commit: "Message drafted",
  Done: "Done",
};

export function chipText(reason: HandbackReason, stage: Stage): string {
  switch (reason.type) {
    case "StageDone":
      return STAGE_DONE[stage];
    case "Replied":
      return reason.first_line;
    case "PermissionNeeded":
      return "Permission needed";
    case "TestFailed":
      return "Tests failed";
    case "Error":
      return "Error";
  }
}

/** The hand-back text for a task that needs you; empty otherwise. */
export function handbackLabel(task: Pick<Task, "stage" | "state">): string {
  return task.state.type === "NeedsYou" ? chipText(task.state.reason, task.stage) : "";
}

/** The primitives' status name for an agent state (`Chip status`, `StatusDot status`). */
export function agentStatus(state: AgentState): AgentStatus {
  if (state.type === "NeedsYou") return "needs-you";
  return state.type === "Working" ? "working" : "idle";
}

/** The primitives' stage token for a stage (`Chip stage`, `Badge stage`). */
export function stageToken(stage: Stage): StageToken {
  return stage.toLowerCase() as StageToken;
}

/** `$1.25`, or `~$1.25` when any part of the figure is estimated. */
export function formatUsd(amount: number, estimated = false): string {
  return `${estimated ? "~" : ""}$${amount.toFixed(2)}`;
}
