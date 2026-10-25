import { describe, it, expect } from "vitest";
import { doneStepsAt, deriveSteps } from "./steps";
import { deriveAgents } from "./attribution";
import { deriveEdits, deriveFiles } from "./files";
import { pairToolCalls } from "./events";
import { fx, sampleBuild, samplePlan, MEMORY_DIR } from "./fixtures";
import type { AgentCost } from "./types";

function steps(events = sampleBuild(), costs: AgentCost[] = []) {
  const agents = deriveAgents(events);
  const plan = samplePlan();
  const edits = deriveEdits(events, MEMORY_DIR, agents, plan);
  return deriveSteps({
    planSteps: plan,
    agents,
    edits,
    files: deriveFiles(edits),
    toolCalls: pairToolCalls(events),
    done: doneStepsAt(events, MEMORY_DIR),
    costs,
  });
}

describe("doneStepsAt", () => {
  it("derives ticked steps from plan.md edits in the stream", () => {
    const events = sampleBuild();
    expect([
      ...doneStepsAt(
        events.filter((e) => e.seq <= 11),
        MEMORY_DIR,
      ),
    ]).toEqual([1]);
    expect([...doneStepsAt(events, MEMORY_DIR)]).toEqual([1, 3]);
  });
  it("ignores plan.md files outside the memory folder", () => {
    expect(doneStepsAt([fx.edit(1, "main", "/repo/app/plan.md", "", "- [x] 1. X")], MEMORY_DIR).size).toBe(0);
  });
});

describe("deriveSteps", () => {
  it("sets done, active and upcoming from ticks, dispatches and edits", () => {
    expect(steps().map((s) => s.status)).toEqual(["done", "active", "done", "active", "upcoming", "upcoming"]);
  });
  it("marks only steps of an open agent as working", () => {
    expect(steps().map((s) => s.working)).toEqual([false, true, false, true, false, false]);
  });
  it("attributes owners as Step N · role roles, dispatch first", () => {
    const plan = samplePlan();
    plan[1] = { ...plan[1], owners: ["frontend"] };
    const events = sampleBuild();
    const agents = deriveAgents(events);
    const edits = deriveEdits(events, MEMORY_DIR, agents, plan);
    const s = deriveSteps({
      planSteps: plan,
      agents,
      edits,
      files: deriveFiles(edits),
      toolCalls: pairToolCalls(events),
      done: new Set(),
      costs: [],
    });
    expect(s[1].owners).toEqual(["ui-polish", "frontend"]);
    expect(s[0].owners).toEqual(["frontend"]);
  });
  it("lists the step's files with +/- and the shared flag", () => {
    // file-level net: +2 at seq 5, +1 −1 at seq 9
    expect(steps()[0].files).toEqual([{ path: "/repo/app/src/a.ts", added: 3, removed: 1, shared: true }]);
  });
  it("shows the latest tool line of the step's agent", () => {
    expect(steps()[2].toolLine).toBe("Bash npm test");
    expect(steps()[5].toolLine).toBeNull();
  });
  it("computes progress as touched planned files over planned files", () => {
    const s = steps();
    expect(s[0].progress).toBe(1); // done
    expect(s[1].progress).toBe(1); // toolbar.tsx touched, 1 of 1
    expect(s[3].progress).toBe(0); // nothing attributed to step 4 yet
    expect(s[5].progress).toBeNull(); // no planned files
  });
  it("splits an agent's cost evenly across its steps", () => {
    const s = steps(sampleBuild(), [{ agentId: "a3", costUsd: 0.4 }]);
    expect(s[1].costUsd).toBeCloseTo(0.2);
    expect(s[3].costUsd).toBeCloseTo(0.2);
    expect(s[0].costUsd).toBeNull();
  });
});
