import { expect, test } from "vitest";
import type { AgentState, Stage, StageAction } from "@/lib/bindings";
import { handbackKind, isWorking, testExitCode } from "./agentState";
import {
  forwardActionFor,
  freshSessionAllowed,
  primaryActionFor,
  type PrimaryCtx,
  type PrimaryIntent,
} from "./primaryAction";

const idle: AgentState = { type: "Idle" },
  working: AgentState = { type: "Working" };
const done: AgentState = { type: "NeedsYou", reason: { type: "StageDone" } };
const replied: AgentState = { type: "NeedsYou", reason: { type: "Replied", first_line: "Which API?" } };
const failed: AgentState = { type: "NeedsYou", reason: { type: "TestFailed", exit_code: 1 } };
const perm: AgentState = { type: "NeedsYou", reason: { type: "PermissionNeeded" } };
const inUse: AgentState = { type: "NeedsYou", reason: { type: "FileInUse", pending: 2 } };
const holding: AgentState = { type: "NeedsYou", reason: { type: "HoldingFor", other: "Login" } };
const merge: AgentState = { type: "NeedsYou", reason: { type: "MergeNeedsYou", repo: "/r", paths: ["a.ts"] } };
const overlapTab: PrimaryIntent = { kind: "dock-tab", tab: "overlap" };
const ctx = { mayCommit: false, queued: false };
const t = (stage: Stage, state: AgentState, archived = false) => ({ stage, state, archived });
const act = (type: StageAction["type"]): PrimaryIntent => ({ kind: "action", action: { type } as StageAction });

test.each<[string, Parameters<typeof primaryActionFor>[0], PrimaryCtx, string, PrimaryIntent, boolean]>([
  ["Spec idle", t("Spec", idle), ctx, "Approve spec", act("Approve"), false],
  ["Plan written", t("Plan", done), ctx, "Start build", act("Advance"), false],
  ["Plan replied", t("Plan", replied), ctx, "Reply", { kind: "focus-terminal" }, false],
  ["Plan working", t("Plan", working), ctx, "Start build", act("Advance"), false],
  ["Build done", t("Build", done), ctx, "Request review", act("RequestReview"), false],
  ["Review done", t("Review", done), ctx, "Approve changes", act("ApproveChanges"), false],
  ["Review permission", t("Review", perm), ctx, "Approve changes", act("ApproveChanges"), false],
  ["Test running", t("Test", working), ctx, "Running tests…", { kind: "none" }, true],
  ["Test failed", t("Test", failed), ctx, "Fix failing tests", act("FixTests"), false],
  ["Test passed", t("Test", done), ctx, "Continue to commit", act("Advance"), false],
  ["Test idle", t("Test", idle), ctx, "Run tests", act("RunTests"), false],
  ["Commit may commit", t("Commit", done), { ...ctx, mayCommit: true }, "Commit", act("Commit"), false],
  ["Commit may not", t("Commit", done), ctx, "Mark done", act("Advance"), false],
  ["Done", t("Done", idle), ctx, "Archive", act("Archive"), false],
  ["Done stage done", t("Done", done), ctx, "Archive", act("Archive"), false],
  ["file in use gives Decide opening the overlap tab", t("Build", inUse), ctx, "Decide", overlapTab, false],
  ["holding for another task gives Decide", t("Build", holding), ctx, "Decide", overlapTab, false],
  ["merge needs you gives Resolve merge", t("Done", merge), ctx, "Resolve merge", overlapTab, false],
  ["archived", t("Build", idle, true), ctx, "Restore", act("Restore"), false],
])("%s", (_n, task, c, label, intent, disabled) => {
  expect(primaryActionFor(task, c)).toMatchObject({ label, intent, disabled });
});
test("a queued action while working shows the hint", () => {
  expect(primaryActionFor(t("Plan", working), { ...ctx, queued: true }).hint).toBe("Will send when idle");
  expect(primaryActionFor(t("Plan", done), ctx).hint).toBeNull();
});
test("freshSessionAllowed excludes Spec and Done", () => {
  expect(
    ["Spec", "Plan", "Build", "Review", "Test", "Commit", "Done"].map((s) => freshSessionAllowed(s as Stage)),
  ).toEqual([false, true, true, true, true, true, false]);
});
test("forwardActionFor ignores state and has nothing for Done", () => {
  expect(forwardActionFor("Plan", ctx)).toEqual({ label: "Start build", action: { type: "Advance" } });
  expect(forwardActionFor("Commit", { mayCommit: true })).toEqual({ label: "Commit", action: { type: "Commit" } });
  expect(forwardActionFor("Done", ctx)).toBeNull();
});
test("handbackKind and helpers read the internally tagged shape", () => {
  expect(handbackKind(replied)).toBe("Replied");
  expect(handbackKind(perm)).toBe("PermissionNeeded");
  expect(testExitCode(failed)).toBe(1);
  expect(handbackKind(idle)).toBeNull();
  expect(isWorking(working)).toBe(true);
  expect(isWorking(idle)).toBe(false);
});

test("Claude Code's own question sends you to the terminal in every stage, Spec included", () => {
  const asks: AgentState = { type: "NeedsYou", reason: { type: "AgentAsks", question: "TrustFolder" } };
  for (const stage of ["Spec", "Plan", "Build"] as Stage[]) {
    const p = primaryActionFor(t(stage, asks), ctx);
    expect(p.label).toBe("Answer in terminal");
    expect(p.intent).toEqual({ kind: "focus-terminal" });
  }
});

test("open blockers in review show on the Approve changes button", () => {
  const review = t("Review", { type: "Idle" });
  expect(primaryActionFor(review, { ...ctx, blockers: 2 }).hint).toBe("2 blockers in review");
  expect(primaryActionFor(review, { ...ctx, blockers: 1 }).hint).toBe("1 blocker in review");
  expect(primaryActionFor(review, ctx).hint).toBeNull();
  expect(primaryActionFor(t("Build", { type: "Idle" }), { ...ctx, blockers: 1 }).hint).toBeNull();
});
