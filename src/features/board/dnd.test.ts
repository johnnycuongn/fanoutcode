import { describe, expect, it, vi } from "vitest";
import {
  boardAnnouncements,
  columnDroppableId,
  handleDragEnd,
  laneDraggableId,
  nextColumnTarget,
  parseDroppableId,
  parseLaneId,
  resolveDrop,
} from "./dnd";
import { makeTask } from "./testing/fixtures";

const star: (t: { project_id: string }) => string = () => "*";
const byProject = (t: { project_id: string }) => t.project_id;
const task = makeTask({ id: "t-1", stage: "Plan", project_id: "p-1" });
const drop = (overId: string | null, laneOf = star, readOnly = false) =>
  resolveDrop({ task, overId, laneOf, readOnly });

describe("resolveDrop", () => {
  it("resolves a forward drop onto a later column", () => {
    expect(drop(columnDroppableId("*", "Build"))).toEqual({ kind: "move", taskId: "t-1", from: "Plan", to: "Build" });
  });
  it("resolves a backward drop onto an earlier column", () => {
    expect(drop(columnDroppableId("*", "Spec"))).toMatchObject({ kind: "move", to: "Spec" });
  });
  it("resolves a drop across several columns", () => {
    expect(drop(columnDroppableId("*", "Commit"))).toMatchObject({ kind: "move", from: "Plan", to: "Commit" });
  });
  it("is a no-op when dropped in the same column", () => {
    expect(drop(columnDroppableId("*", "Plan"))).toEqual({ kind: "rejected", reason: "same_column" });
  });
  it("rejects a drop onto another lane", () => {
    expect(drop(columnDroppableId("p-2", "Build"), byProject)).toEqual({ kind: "rejected", reason: "cross_lane" });
    expect(drop(columnDroppableId("p-1", "Build"), byProject)).toMatchObject({ kind: "move" });
  });
  it("rejects every drop when the board is read-only", () => {
    expect(drop(columnDroppableId("*", "Build"), star, true)).toEqual({ kind: "rejected", reason: "read_only" });
  });
  it("returns none without an over target, with an unparsable id, or when the task vanished", () => {
    // Review Focus 3
    expect(drop(null)).toEqual({ kind: "none" });
    expect(drop("garbage")).toEqual({ kind: "none" });
    expect(
      resolveDrop({ task: undefined, overId: columnDroppableId("*", "Build"), laneOf: star, readOnly: false }),
    ).toEqual({ kind: "none" });
  });
});

describe("ids", () => {
  it("round-trips column ids and rejects lane ids", () => {
    expect(parseDroppableId(columnDroppableId("p-1", "Test"))).toEqual({ lane: "p-1", stage: "Test" });
    expect(parseDroppableId(laneDraggableId("p-1"))).toBeNull();
    expect(parseDroppableId("col:*:Nowhere")).toBeNull();
    expect(parseLaneId(laneDraggableId("p-1"))).toBe("p-1");
    expect(parseLaneId(columnDroppableId("p-1", "Test"))).toBeNull();
  });
});

describe("nextColumnTarget", () => {
  const rects = new Map([
    [columnDroppableId("*", "Spec"), { left: 0, top: 0, width: 100, height: 400 }],
    [columnDroppableId("*", "Plan"), { left: 100, top: 0, width: 100, height: 400 }],
    [columnDroppableId("p-2", "Build"), { left: 200, top: 500, width: 100, height: 400 }],
  ]);
  it("ArrowRight moves to the next column in the same lane and ArrowLeft at Spec stays", () => {
    expect(nextColumnTarget("right", { lane: "*", stage: "Spec" }, rects)).toEqual({ x: 150, y: 200 });
    expect(nextColumnTarget("left", { lane: "*", stage: "Spec" }, rects)).toBeUndefined();
    expect(nextColumnTarget("right", { lane: "*", stage: "Plan" }, rects)).toBeUndefined(); // Build exists only in lane p-2
  });
  it("ArrowLeft moves back one column", () => {
    expect(nextColumnTarget("left", { lane: "*", stage: "Plan" }, rects)).toEqual({ x: 50, y: 200 });
  });
});

describe("handleDragEnd", () => {
  const deps = () => ({
    tasks: { "t-1": task },
    laneOf: star,
    readOnly: false,
    laneOrder: ["p-1", "p-2"],
    moveTask: vi.fn(async () => {}),
    reorderLanes: vi.fn(),
  });
  it("calls moveTask for a move outcome and nothing otherwise", () => {
    const d = deps();
    handleDragEnd({ active: { id: "t-1" } as never, over: { id: columnDroppableId("*", "Build") } as never }, d);
    expect(d.moveTask).toHaveBeenCalledWith("t-1", "Build");
    handleDragEnd({ active: { id: "t-1" } as never, over: { id: columnDroppableId("*", "Plan") } as never }, d);
    handleDragEnd({ active: { id: "gone" } as never, over: { id: columnDroppableId("*", "Build") } as never }, d);
    handleDragEnd({ active: { id: "t-1" } as never, over: null }, d);
    expect(d.moveTask).toHaveBeenCalledTimes(1);
  });
  it("reorders lanes for a lane drag", () => {
    const d = deps();
    handleDragEnd(
      { active: { id: laneDraggableId("p-2") } as never, over: { id: laneDraggableId("p-1") } as never },
      d,
    );
    expect(d.reorderLanes).toHaveBeenCalledWith(1, 0);
  });
  it("a lane dropped on itself or on nothing reorders nothing", () => {
    const d = deps();
    handleDragEnd(
      { active: { id: laneDraggableId("p-2") } as never, over: { id: laneDraggableId("p-2") } as never },
      d,
    );
    handleDragEnd({ active: { id: laneDraggableId("p-2") } as never, over: null }, d);
    expect(d.reorderLanes).not.toHaveBeenCalled();
  });
});

describe("boardAnnouncements", () => {
  it("names the task and stage", () => {
    const a = boardAnnouncements((id) => (id === "t-1" ? "Add login" : id));
    expect(a.onDragStart!({ active: { id: "t-1", data: { current: { stage: "Plan" } } } } as never)).toBe(
      "Picked up Add login from Plan",
    );
    expect(a.onDragOver!({ active: { id: "t-1" }, over: { id: columnDroppableId("*", "Build") } } as never)).toBe(
      "Add login is over Build",
    );
    expect(a.onDragEnd!({ active: { id: "t-1" }, over: { id: columnDroppableId("*", "Build") } } as never)).toBe(
      "Add login dropped in Build",
    );
    expect(a.onDragCancel!({ active: { id: "t-1" } } as never)).toBe("Dragging Add login cancelled");
  });
});
