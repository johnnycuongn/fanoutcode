import { describe, expect, it } from "vitest";
import { STAGES, groupByLane, groupByStage, isForward, orderWithinColumn, stageIndex } from "./columns";
import { IDLE, WORKING, makeTask, needsYou } from "./testing/fixtures";

describe("groupByStage", () => {
  it("always yields the seven stages in pipeline order even when empty", () => {
    const cols = groupByStage([]);
    expect(cols.map((c) => c.stage)).toEqual(STAGES);
    expect(cols.map((c) => c.count)).toEqual([0, 0, 0, 0, 0, 0, 0]);
  });
  it("places each task in its stage column and counts it", () => {
    const cols = groupByStage([
      makeTask({ id: "a", stage: "Build" }),
      makeTask({ id: "b", stage: "Build" }),
      makeTask({ id: "c", stage: "Done" }),
    ]);
    expect(cols[stageIndex("Build")]!.count).toBe(2);
    expect(cols[stageIndex("Done")]!.tasks.map((t) => t.id)).toEqual(["c"]);
  });
});

describe("orderWithinColumn", () => {
  it("orders needs-you before working before idle", () => {
    const out = orderWithinColumn([
      makeTask({ id: "i", state: IDLE }),
      makeTask({ id: "w", state: WORKING }),
      makeTask({ id: "n", state: needsYou({ type: "StageDone" }) }),
    ]);
    expect(out.map((t) => t.id)).toEqual(["n", "w", "i"]);
  });
  it("orders flagged before unflagged within the same state", () => {
    const out = orderWithinColumn([makeTask({ id: "a" }), makeTask({ id: "b", flagged: true })]);
    expect(out.map((t) => t.id)).toEqual(["b", "a"]);
  });
  it("orders by priority High to None within the same state and flag", () => {
    const out = orderWithinColumn([
      makeTask({ id: "n", priority: "None" }),
      makeTask({ id: "h", priority: "High" }),
      makeTask({ id: "m", priority: "Medium" }),
      makeTask({ id: "l", priority: "Low" }),
    ]);
    expect(out.map((t) => t.id)).toEqual(["h", "m", "l", "n"]);
  });
  it("orders newest updated_at first as the last tie-break", () => {
    const out = orderWithinColumn([
      makeTask({ id: "old", updated_at: "2026-10-24T09:00:00Z" }),
      makeTask({ id: "new", updated_at: "2026-10-24T11:00:00Z" }),
    ]);
    expect(out.map((t) => t.id)).toEqual(["new", "old"]);
  });
});

describe("groupByLane", () => {
  it("follows the lane order and keeps empty lanes", () => {
    const lanes = groupByLane(
      [makeTask({ id: "a", project_id: "p-2" }), makeTask({ id: "x", project_id: "p-9" })],
      ["p-1", "p-2"],
    );
    expect([...lanes.keys()]).toEqual(["p-1", "p-2"]);
    expect(lanes.get("p-1")).toEqual([]);
    expect(lanes.get("p-2")!.map((t) => t.id)).toEqual(["a"]);
  });
});

describe("isForward", () => {
  it("is true for Spec→Build and false for Build→Plan and Plan→Plan", () => {
    expect(isForward("Spec", "Build")).toBe(true);
    expect(isForward("Build", "Plan")).toBe(false);
    expect(isForward("Plan", "Plan")).toBe(false);
  });
});
