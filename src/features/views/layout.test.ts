import { describe, it, expect } from "vitest";
import {
  leaf,
  split,
  EMPTY_PANE,
  taskContent,
  teamCanvasContent,
  terminalContent,
  sameContent,
  newPaneId,
  leaves,
  splitPane,
  removePane,
  setRatio,
  setContent,
  movePane,
  rects,
  neighbour,
} from "./layout";
import type { SplitLayout } from "./types";

const A = leaf("pane-aa", taskContent("task-b2")),
  B = leaf("pane-bb", EMPTY_PANE),
  C = leaf("pane-cc", EMPTY_PANE);
// [A | [B / C]] : A left half, B top-right, C bottom-right
const tree = split("Horizontal", 0.5, A, split("Vertical", 0.5, B, C));

describe("layout", () => {
  it("generates path-safe pane ids", () => expect(newPaneId()).toMatch(/^pane-[0-9a-f]{8}$/));
  it("lists leaves with paths", () => {
    expect(leaves(tree).map((l) => [l.pane, l.path])).toEqual([
      ["pane-aa", ["first"]],
      ["pane-bb", ["second", "first"]],
      ["pane-cc", ["second", "second"]],
    ]);
  });
  it("builds internally tagged nodes", () => {
    expect(B).toEqual({ type: "Leaf", pane: "pane-bb", content: { type: "Empty" } });
    expect(tree).toMatchObject({ type: "Split", direction: "Horizontal", ratio: 0.5 });
    expect(taskContent("task-b2")).toEqual({ type: "Task", task_id: "task-b2" });
    expect(teamCanvasContent("task-b2")).toEqual({ type: "TeamCanvas", task_id: "task-b2" });
    expect(terminalContent("proj-a1", "sess-9f")).toEqual({
      type: "Terminal",
      project_id: "proj-a1",
      session_id: "sess-9f",
    });
    expect(sameContent(teamCanvasContent("task-b2"), { type: "TeamCanvas", task_id: "task-b2" })).toBe(true);
    expect(sameContent(teamCanvasContent("task-b2"), taskContent("task-b2"))).toBe(false);
  });
  it("splits a leaf into a 0.5 split with an Empty second pane, or given content", () => {
    const out = splitPane(A, "pane-aa", "Horizontal", "pane-dd");
    expect(out).toEqual(split("Horizontal", 0.5, A, leaf("pane-dd", EMPTY_PANE)));
    expect(splitPane(A, "pane-aa", "Horizontal", "pane-dd", teamCanvasContent("task-b2"))).toEqual(
      split("Horizontal", 0.5, A, leaf("pane-dd", teamCanvasContent("task-b2"))),
    );
  });
  it("removes a pane so its sibling takes the parent slot", () => {
    expect(removePane(tree, "pane-bb")).toEqual(split("Horizontal", 0.5, A, C));
    expect(removePane(A, "pane-aa")).toEqual(leaf("pane-aa", EMPTY_PANE));
  });
  it("clamps ratios", () => {
    expect((setRatio(tree, [], 0.02) as SplitLayout).ratio).toBe(0.1);
    expect(((setRatio(tree, ["second"], 0.95) as SplitLayout).second as SplitLayout).ratio).toBe(0.9);
  });
  it("sets content", () =>
    expect(leaves(setContent(tree, "pane-bb", taskContent("task-99")))[1].content).toEqual(taskContent("task-99")));
  it("moves a pane to the top edge of another", () => {
    const out = movePane(tree, "pane-cc", "pane-aa", "top");
    expect(out).toEqual(split("Horizontal", 0.5, split("Vertical", 0.5, C, A), B));
    expect(movePane(tree, "pane-aa", "pane-aa", "left")).toBe(tree);
  });
  it("computes rects and neighbours", () => {
    const r = rects(tree);
    expect(r.get("pane-cc")).toEqual({ x: 0.5, y: 0.5, w: 0.5, h: 0.5 });
    expect(neighbour(tree, "pane-aa", "right")).toBe("pane-bb");
    expect(neighbour(tree, "pane-cc", "up")).toBe("pane-bb");
    expect(neighbour(tree, "pane-cc", "left")).toBe("pane-aa");
    expect(neighbour(tree, "pane-aa", "left")).toBeNull();
  });
});
