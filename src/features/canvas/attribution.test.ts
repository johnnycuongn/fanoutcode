import { describe, it, expect } from "vitest";
import { parseDispatch, deriveAgents, mainWorking } from "./attribution";
import { fx, sampleBuild } from "./fixtures";

describe("parseDispatch", () => {
  it("parses a single step and role", () => {
    expect(parseDispatch("Step 5 · frontend: wire the toolbar")).toEqual({ steps: [5], role: "frontend" });
  });
  it("parses several steps", () => {
    expect(parseDispatch("Step 5, 6 · frontend: wire it")).toEqual({ steps: [5, 6], role: "frontend" });
  });
  it("returns null without the marker", () => {
    expect(parseDispatch("Investigate flaky test")).toBeNull();
  });
  it("accepts a hyphen or em dash separator and is case-insensitive", () => {
    expect(parseDispatch("step 2 - Backend: add route")).toEqual({ steps: [2], role: "backend" });
    expect(parseDispatch("Step 2 — backend: add route")).toEqual({ steps: [2], role: "backend" });
  });
});

describe("deriveAgents", () => {
  it("always lists the orchestrator first as main", () => {
    expect(deriveAgents([])[0]).toMatchObject({ agentId: "main", role: "orchestrator", working: false });
  });
  it("creates an agent per SubagentStart with role and steps from the dispatch", () => {
    const a1 = deriveAgents(sampleBuild()).find((a) => a.agentId === "a1")!;
    expect(a1).toMatchObject({
      role: "frontend",
      steps: [1],
      adhoc: false,
      working: false,
      startedSeq: 4,
      stoppedSeq: 10,
    });
  });
  it("marks a general-purpose agent with a dispatch role as ad hoc and still working", () => {
    const a3 = deriveAgents(sampleBuild()).find((a) => a.agentId === "a3")!;
    expect(a3).toMatchObject({ role: "ui-polish", steps: [2, 4], adhoc: true, working: true, stoppedSeq: null });
  });
  it("falls back to the pending Task dispatch when SubagentStart has no description", () => {
    const events = [
      fx.pre(1, "main", "Task", { description: "Step 2 · backend: x" }),
      fx.agentStart(2, "b", "backend"),
    ];
    expect(deriveAgents(events).find((a) => a.agentId === "b")).toMatchObject({ steps: [2], role: "backend" });
  });
  it("creates a dispatch agent when a Task call completes without SubagentStart", () => {
    const events = [
      fx.pre(1, "main", "Task", { description: "Step 2 · backend: x" }),
      fx.post(2, "main", "Task", { description: "Step 2 · backend: x" }),
    ];
    expect(deriveAgents(events).find((a) => a.agentId === "dispatch:1")).toMatchObject({
      role: "backend",
      steps: [2],
      startedSeq: 1,
      stoppedSeq: 2,
      working: false,
    });
  });
  it("ignores a SubagentStop without a matching start", () => {
    expect(deriveAgents([fx.agentStop(1, "ghost")])).toHaveLength(1);
  });
});

describe("mainWorking", () => {
  it("is true after a prompt and false after a plain Stop", () => {
    expect(mainWorking([fx.prompt(1)])).toBe(true);
    expect(mainWorking([fx.prompt(1), fx.stop(2, "done")])).toBe(false);
  });
  it("stays true when a Stop still lists background tasks", () => {
    expect(mainWorking([fx.prompt(1), fx.stop(2, "wait", 2)])).toBe(true);
  });
});
