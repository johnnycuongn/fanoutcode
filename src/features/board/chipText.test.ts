import { describe, expect, it } from "vitest";
import { chipFor, firstLine } from "./chipText";
import { IDLE, WORKING, makeTask, needsYou } from "./testing/fixtures";

describe("chipFor", () => {
  it("shows Working in the working tone", () => {
    expect(chipFor(makeTask({ state: WORKING }))).toEqual({ tone: "working", text: "Working" });
  });
  it("shows Idle in the idle tone", () => {
    expect(chipFor(makeTask({ state: IDLE }))).toEqual({ tone: "idle", text: "Idle" });
  });
  it.each([
    ["Spec", "Spec ready", "needs_you"],
    ["Plan", "Plan written, go?", "needs_you"],
    ["Build", "Build done", "needs_you"],
    ["Review", "Review written", "needs_you"],
    ["Test", "Tests passed", "ok"],
    ["Commit", "Message drafted", "needs_you"],
    ["Done", "Done", "ok"],
  ] as const)('StageDone in %s reads "%s"', (stage, text, tone) => {
    expect(chipFor(makeTask({ stage, state: needsYou({ type: "StageDone" }) }))).toEqual({ tone, text });
  });
  it('Replied reads "Your turn", whatever the agent wrote', () => {
    // the agent's words stay in the terminal; a reply to an app notice once took over the chip
    const long = "Noted. My build is finished, so the undo task's start doesn't affect anything I'm doing.";
    expect(chipFor(makeTask({ state: needsYou({ type: "Replied", first_line: long }) }))).toEqual({
      tone: "needs_you",
      text: "Your turn",
    });
  });
  it('PermissionNeeded reads "Permission needed"', () => {
    expect(chipFor(makeTask({ state: needsYou({ type: "PermissionNeeded" }) })).text).toBe("Permission needed");
  });
  it('TestFailed reads "Tests failed" with the exit code as detail', () => {
    expect(chipFor(makeTask({ stage: "Test", state: needsYou({ type: "TestFailed", exit_code: 1 }) }))).toEqual({
      tone: "needs_you",
      text: "Tests failed",
      detail: "exit 1",
    });
  });
  it("Error carries the message as detail", () => {
    expect(chipFor(makeTask({ state: needsYou({ type: "Error", message: "spawn failed" }) }))).toEqual({
      tone: "needs_you",
      text: "Error",
      detail: "spawn failed",
    });
  });
});

describe("firstLine", () => {
  it("returns the first non-empty line, trimmed, with an ellipsis past max", () => {
    expect(firstLine("\n  hello \nworld")).toBe("hello");
    expect(firstLine("x".repeat(90), 80)).toBe("x".repeat(79) + "…");
  });
});
