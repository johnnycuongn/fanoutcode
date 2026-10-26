import { describe, it, expect } from "vitest";
import fixture from "@/test/fixtures/handback.json";
import type { HandbackReason, Stage } from "@/lib/bindings";
import { makeTask, needsYou, WORKING } from "@/test/fixtures";
import { chipText, handbackLabel } from "./format";

describe("hand-back chip text", () => {
  it("uses the canonical StageDone text for every stage", () => {
    const want: Record<Stage, string> = {
      Spec: "Spec ready",
      Plan: "Plan written, go?",
      Build: "Build done",
      Review: "Review written",
      Test: "Tests passed",
      Commit: "Message drafted",
      Done: "Done",
    };
    for (const [stage, text] of Object.entries(want))
      expect(chipText({ type: "StageDone" }, stage as Stage)).toBe(text);
  });
  it("labels the other reasons from the task, and is empty outside needs-you", () => {
    expect(handbackLabel(makeTask({ stage: "Plan", state: needsYou() }))).toBe("Plan written, go?");
    expect(handbackLabel(makeTask({ state: needsYou({ type: "Replied", first_line: "Which DB?" }) }))).toBe(
      "Your turn",
    );
    expect(handbackLabel(makeTask({ state: needsYou({ type: "PermissionNeeded" }) }))).toBe("Permission needed");
    expect(handbackLabel(makeTask({ state: needsYou({ type: "TestFailed", exit_code: 2 }) }))).toBe("Tests failed");
    expect(handbackLabel(makeTask({ state: needsYou({ type: "Error", message: "boom" }) }))).toBe("Error");
    expect(handbackLabel(makeTask({ state: needsYou({ type: "FileInUse", pending: 1 }) }))).toBe("File in use");
    expect(handbackLabel(makeTask({ state: needsYou({ type: "FileInUse", pending: 3 }) }))).toBe("3 files in use");
    expect(handbackLabel(makeTask({ state: needsYou({ type: "HoldingFor", other: "Login" }) }))).toBe(
      "Holding for Login",
    );
    expect(handbackLabel(makeTask({ state: needsYou({ type: "MergeNeedsYou", repo: "/r", paths: [] }) }))).toBe(
      "Merge needs you",
    );
    expect(handbackLabel(makeTask({ state: WORKING }))).toBe("");
  });
  it("matches the Rust chip_text fixture row for row", () => {
    const rows = fixture as Array<{ reason: HandbackReason; stage: Stage; text: string }>;
    expect(rows).toHaveLength(84);
    for (const { reason, stage, text } of rows) expect(chipText(reason, stage)).toBe(text);
  });
});
