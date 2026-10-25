import { describe, it, expect } from "vitest";
import { layoutTrigger, resolvePositions, prunePositions, stackDone } from "./positions";

const p = (node_id: string, x: number, y: number) => ({ node_id, x, y });
const previous = [p("step:1", 0, 0), p("step:2", 100, 0), p("step:9", 9, 9)];
const laidOut = [p("step:1", 10, 10), p("step:2", 110, 10), p("step:3", 210, 10)];
const nodeIds = ["step:1", "step:2", "step:3"];

describe("layoutTrigger", () => {
  it("is initial with nothing saved, build-start on entering Build, change otherwise", () => {
    expect(layoutTrigger(null, "Build", false)).toBe("initial");
    expect(layoutTrigger("Plan", "Build", true)).toBe("build-start");
    expect(layoutTrigger("Build", "Build", true)).toBe("change");
    expect(layoutTrigger(null, "Build", true)).toBe("change");
  });
});

describe("resolvePositions", () => {
  it("auto re-flow replaces positions on every change", () => {
    expect(resolvePositions({ mode: "AutoReflow", trigger: "change", previous, laidOut, nodeIds })).toEqual(laidOut);
  });
  it("pinned keeps existing positions on change and places only new nodes", () => {
    expect(resolvePositions({ mode: "Pinned", trigger: "change", previous, laidOut, nodeIds })).toEqual([
      p("step:1", 0, 0),
      p("step:2", 100, 0),
      p("step:3", 210, 10),
    ]);
  });
  it("pinned re-lays out everything on tidy, build-start and initial", () => {
    for (const trigger of ["tidy", "build-start", "initial"] as const) {
      expect(resolvePositions({ mode: "Pinned", trigger, previous, laidOut, nodeIds })).toEqual(laidOut);
    }
  });
});

describe("prunePositions", () => {
  it("drops ids that are not in the graph", () => {
    expect(prunePositions(previous, nodeIds).map((x) => x.node_id)).toEqual(["step:1", "step:2"]);
  });
});

describe("stackDone", () => {
  it("places done steps at the anchor with 8 px offsets and leaves others alone", () => {
    const out = stackDone(laidOut, ["step:1", "step:3"], { x: 0, y: 0 });
    expect(out).toEqual([p("step:1", 0, 0), p("step:2", 110, 10), p("step:3", 8, 8)]);
  });
});
