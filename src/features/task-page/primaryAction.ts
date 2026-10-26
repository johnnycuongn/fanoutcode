// The task header's primary button per stage and hand-back state (§6). Button copy is this plan's; the chip text is
// `handbackLabel` (A2). Plan 06's palette and Plan 07's card menu use `forwardActionFor` so every surface agrees.
import type { Stage, StageAction, Task } from "@/lib/bindings";
import type { DockTab } from "@/features/dock";
import { handbackKind, isWorking } from "./agentState";

export type PrimaryIntent =
  | { kind: "action"; action: StageAction }
  | { kind: "focus-terminal" }
  | { kind: "dock-tab"; tab: DockTab }
  | { kind: "none" };
export interface PrimaryButton {
  label: string;
  intent: PrimaryIntent;
  disabled: boolean;
  hint: string | null;
}
export interface PrimaryCtx {
  mayCommit: boolean;
  queued: boolean;
  /** Blockers review.md lists (Review only); shown on the button before Approve changes. */
  blockers?: number;
}

export const QUEUED_HINT = "Will send when idle";

export const PREVIOUS_STAGE: Partial<Record<Stage, Stage>> = {
  Plan: "Spec",
  Build: "Plan",
  Review: "Build",
  Test: "Review",
  Commit: "Test",
  Done: "Commit",
};

const action = (type: Exclude<StageAction["type"], "Back" | "Reply">): StageAction => ({ type }) as StageAction;

/** The stage's forward step, whatever the agent is doing; null for Done. */
export function forwardActionFor(
  stage: Stage,
  ctx: Pick<PrimaryCtx, "mayCommit">,
): { label: string; action: StageAction } | null {
  switch (stage) {
    case "Spec":
      return { label: "Approve spec", action: action("Approve") };
    case "Plan":
      return { label: "Start build", action: action("Advance") };
    case "Build":
      return { label: "Request review", action: action("RequestReview") };
    case "Review":
      return { label: "Approve changes", action: action("ApproveChanges") };
    case "Test":
      return { label: "Continue to commit", action: action("Advance") };
    case "Commit":
      return ctx.mayCommit
        ? { label: "Commit", action: action("Commit") }
        : { label: "Mark done", action: action("Advance") };
    case "Done":
      return null;
  }
}

/** Plan through Commit: Spec has no agent session to restart, Done has nothing left to do. */
export function freshSessionAllowed(stage: Stage): boolean {
  return stage !== "Spec" && stage !== "Done";
}

const button = (label: string, intent: PrimaryIntent, disabled = false): Omit<PrimaryButton, "hint"> => ({
  label,
  intent,
  disabled,
});

function decide(task: Pick<Task, "stage" | "state" | "archived">, ctx: PrimaryCtx): Omit<PrimaryButton, "hint"> {
  if (task.archived) return button("Restore", { kind: "action", action: action("Restore") });
  const kind = handbackKind(task.state);
  // Claude Code's own question (trust, sign-in) is answered in its terminal, whatever the stage
  if (kind === "AgentAsks") return button("Answer in terminal", { kind: "focus-terminal" });
  if (kind === "Replied" && task.stage !== "Spec" && task.stage !== "Done")
    return button("Reply", { kind: "focus-terminal" });
  // Plan 11: an overlap decision or a stuck merge is settled in the Overlap tab, whatever the stage (a Done card can
  // be needs-you while it brings its base in).
  if (kind === "FileInUse" || kind === "HoldingFor") return button("Decide", { kind: "dock-tab", tab: "overlap" });
  if (kind === "MergeNeedsYou") return button("Resolve merge", { kind: "dock-tab", tab: "overlap" });
  if (task.stage === "Done") return button("Archive", { kind: "action", action: action("Archive") });
  if (task.stage === "Test") {
    if (isWorking(task.state)) return button("Running tests…", { kind: "none" }, true);
    if (kind === "TestFailed") return button("Fix failing tests", { kind: "action", action: action("FixTests") });
    if (kind !== "StageDone") return button("Run tests", { kind: "action", action: action("RunTests") });
  }
  const forward = forwardActionFor(task.stage, ctx)!;
  return button(forward.label, { kind: "action", action: forward.action });
}

export function primaryActionFor(task: Pick<Task, "stage" | "state" | "archived">, ctx: PrimaryCtx): PrimaryButton {
  const b = ctx.blockers ?? 0;
  const hint =
    ctx.queued && isWorking(task.state)
      ? QUEUED_HINT
      : task.stage === "Review" && b > 0
        ? `${b} ${b === 1 ? "blocker" : "blockers"} in review`
        : null;
  return { ...decide(task, ctx), hint };
}
