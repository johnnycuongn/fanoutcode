import { beforeEach, describe, expect, it } from "vitest";
import { laneStorageKey, loadLaneState, moveLane, reconcileLanes, saveLaneState } from "./lanes";
import { workspaceContainer } from "./testing/fixtures";

describe("lanes", () => {
  beforeEach(() => localStorage.clear());
  it("loadLaneState keeps saved order, drops unknown ids and appends new ones", () => {
    saveLaneState("k", { order: ["p-2", "p-1", "p-gone"], collapsed: ["p-1", "p-gone"] });
    expect(loadLaneState("k", ["p-1", "p-2", "p-3"])).toEqual({ order: ["p-2", "p-1", "p-3"], collapsed: ["p-1"] });
  });
  it("falls back to the project order on missing or corrupt storage", () => {
    expect(loadLaneState("none", ["p-1", "p-2"])).toEqual({ order: ["p-1", "p-2"], collapsed: [] });
    localStorage.setItem("bad", "{not json");
    expect(loadLaneState("bad", ["p-1"])).toEqual({ order: ["p-1"], collapsed: [] });
    localStorage.setItem("odd", JSON.stringify({ order: "nope", collapsed: [1, 2] }));
    expect(loadLaneState("odd", ["p-1"])).toEqual({ order: ["p-1"], collapsed: [] });
  });
  it("moveLane reorders without mutating", () => {
    const order = ["a", "b", "c"];
    expect(moveLane(order, 2, 0)).toEqual(["c", "a", "b"]);
    expect(order).toEqual(["a", "b", "c"]);
  });
  it("moveLane ignores an index outside the list", () => {
    expect(moveLane(["a", "b"], -1, 0)).toEqual(["a", "b"]);
    expect(moveLane(["a", "b"], 0, 5)).toEqual(["a", "b"]);
  });
  it("reconcileLanes follows a changed project set", () => {
    expect(reconcileLanes({ order: ["p-2", "p-1"], collapsed: ["p-2"] }, ["p-1", "p-3"])).toEqual({
      order: ["p-1", "p-3"],
      collapsed: [],
    });
  });
  it("laneStorageKey is per container", () => {
    expect(laneStorageKey(workspaceContainer("ws-1"))).toBe("fanoutcode.board.lanes.Workspace.ws-1");
  });
  it("saving never throws when storage is blocked", () => {
    const original = Storage.prototype.setItem;
    Storage.prototype.setItem = () => {
      throw new Error("blocked");
    };
    try {
      expect(() => saveLaneState("k", { order: [], collapsed: [] })).not.toThrow();
    } finally {
      Storage.prototype.setItem = original;
    }
  });
});
