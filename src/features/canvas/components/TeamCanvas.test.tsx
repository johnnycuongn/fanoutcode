import "../../../test/reactFlow";
import { describe, it, expect, vi } from "vitest";
import { screen } from "@testing-library/react";
import { renderUi } from "@/test/ui";
import { ROOTS, TASK_ID, sampleBuild, samplePlan, MEMORY_DIR } from "../fixtures";
import { deriveCanvas } from "../model";
import { toGraph } from "../graph";

vi.mock("../store", async () => {
  const fx = await import("../fixtures");
  const m = (await import("../model")).deriveCanvas({
    steps: fx.samplePlan(),
    events: fx.sampleBuild(),
    memoryDir: fx.MEMORY_DIR,
    roots: fx.ROOTS,
  });
  const g = (await import("../graph")).toGraph(m, "HubGraph");
  const state = {
    taskId: fx.TASK_ID,
    task: { workspace_id: null },
    roots: fx.ROOTS,
    model: m,
    graph: g,
    positions: g.nodes.map((n, i) => ({ node_id: n.id, x: i * 300, y: 0 })),
    settings: {
      detail: "Simple",
      showCost: false,
      arrangement: "HubGraph",
      layout: "AutoReflow",
      lens: "ByFile",
      placement: "DockGrows",
    },
    selection: null,
    replaySeq: null,
    viewMode: "canvas",
  };
  return {
    useCanvas: () => state,
    moveNode: vi.fn(),
    select: vi.fn(),
  };
});
const { TeamCanvas, toFlowElements, handlesFor } = await import("./TeamCanvas");

describe("TeamCanvas", () => {
  it("renders one card per graph node and a minimap", async () => {
    const { container } = renderUi(<TeamCanvas taskId={TASK_ID} />);
    await screen.findAllByText("ui-polish");
    expect(container.querySelectorAll("[data-node-kind]")).toHaveLength(
      toGraph(
        deriveCanvas({ steps: samplePlan(), events: sampleBuild(), memoryDir: MEMORY_DIR, roots: ROOTS }),
        "HubGraph",
      ).nodes.length,
    );
    expect(container.querySelector(".react-flow__minimap")).toBeTruthy();
  });
  it("stacks done steps in auto re-flow and resolves owner roles for file avatars", () => {
    const model = deriveCanvas({ steps: samplePlan(), events: sampleBuild(), memoryDir: MEMORY_DIR, roots: ROOTS });
    const graph = toGraph(model, "HubGraph");
    const settings = { detail: "Simple", showCost: false, layout: "AutoReflow", arrangement: "HubGraph" } as const;
    const state = { model, graph, settings, task: { workspace_id: null }, roots: ROOTS } as never;
    const { nodes, edges } = toFlowElements(state, [], true);
    const done = nodes.filter((n) => n.type === "step" && n.data.status === "done");
    expect(done.map((n) => n.type === "step" && [n.data.stackIndex, n.data.stackCount])).toEqual([
      [0, 2],
      [1, 2],
    ]);
    const file = nodes.find((n) => n.id === "file:/repo/app/src/a.ts")!;
    expect(file.type === "file" && file.data.ownerRoles).toMatchObject({ a1: "frontend", a2: "backend" });
    expect(edges.filter((e) => e.data?.working)).toHaveLength(3);
    expect(edges.every((e) => e.sourceHandle?.endsWith("-out") && e.targetHandle?.endsWith("-in"))).toBe(true);
    const flow = { ...(state as object), settings: { ...settings, arrangement: "Flow" } } as never;
    expect(toFlowElements(flow, [], true).nodes.some((n) => n.type === "step" && n.data.stackIndex !== null)).toBe(
      false,
    );
  });
  it("routes edges between the sides that face each other", () => {
    const p = (x: number, y: number) => ({ node_id: "n", x, y });
    expect(handlesFor({ kind: "depends" }, p(0, 0), p(0, 500))).toEqual({
      sourceHandle: "right-out",
      targetHandle: "left-in",
    });
    expect(handlesFor({ kind: "touches" }, p(0, 400), p(0, 0))).toEqual({
      sourceHandle: "top-out",
      targetHandle: "bottom-in",
    });
    expect(handlesFor({ kind: "touches" }, p(0, 0), p(600, 20))).toEqual({
      sourceHandle: "right-out",
      targetHandle: "left-in",
    });
  });
});
