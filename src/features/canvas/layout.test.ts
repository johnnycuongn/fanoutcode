import { describe, it, expect } from "vitest";
import { layoutGraph, nodeSize, dockAgents, NODE_SIZE } from "./layout";
import { deriveCanvas } from "./model";
import { toGraph } from "./graph";
import { sampleBuild, samplePlan, ROOTS, MEMORY_DIR } from "./fixtures";
import type { CanvasGraph, CanvasPosition } from "./types";

const model = deriveCanvas({ steps: samplePlan(), events: sampleBuild(), memoryDir: MEMORY_DIR, roots: ROOTS });
const hub = toGraph(model, "HubGraph");
const flow = toGraph(model, "Flow");
const at = (pos: CanvasPosition[], id: string) => pos.find((p) => p.node_id === id)!;
function overlaps(pos: CanvasPosition[], g: CanvasGraph) {
  const boxes = g.nodes.map((n) => ({ id: n.id, ...at(pos, n.id), ...nodeSize(n) }));
  const out: string[] = [];
  for (const a of boxes)
    for (const b of boxes)
      if (a.id < b.id && a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height)
        out.push(`${a.id}/${b.id}`);
  return out;
}

describe("layoutGraph", () => {
  it("positions every hub node without overlap", async () => {
    const pos = await layoutGraph(hub, "HubGraph", { stackDone: false });
    expect(pos).toHaveLength(hub.nodes.length);
    expect(overlaps(pos, hub)).toEqual([]);
  });
  it("stacks done steps at the anchor with 8 px offsets when stackDone is on", async () => {
    const pos = await layoutGraph(hub, "HubGraph", { stackDone: true });
    const s1 = at(pos, "step:1"),
      s3 = at(pos, "step:3");
    expect([s3.x - s1.x, s3.y - s1.y]).toEqual([8, 8]);
  });
  it("places a dependent step to the right of its dependencies in the flow canvas", async () => {
    const pos = await layoutGraph(flow, "Flow", { stackDone: false });
    expect(at(pos, "step:4").x).toBeGreaterThan(at(pos, "step:2").x);
    expect(at(pos, "step:4").x).toBeGreaterThan(at(pos, "step:3").x);
    expect(at(pos, "step:3").x).toBeGreaterThan(at(pos, "step:1").x);
  });
  it("docks agents below every step card in the flow canvas", async () => {
    const pos = await layoutGraph(flow, "Flow", { stackDone: false });
    const bottom = Math.max(
      ...flow.nodes.filter((n) => n.kind === "step").map((n) => at(pos, n.id).y + nodeSize(n).height),
    );
    for (const a of flow.nodes.filter((n) => n.kind === "agent"))
      expect(at(pos, a.id).y).toBeGreaterThanOrEqual(bottom + 80);
  });
  it("keeps docked agents from overlapping", () => {
    const steps = flow.nodes.filter((n) => n.kind === "step").map((n, i) => ({ node_id: n.id, x: 0, y: i * 10 }));
    const xs = dockAgents(steps, flow)
      .sort((a, b) => a.x - b.x)
      .map((p) => p.x);
    for (let i = 1; i < xs.length; i++) expect(xs[i] - xs[i - 1]).toBeGreaterThanOrEqual(NODE_SIZE.agent.width + 16);
  });
  it("collapses done and upcoming steps to the title-line height", () => {
    expect(nodeSize(flow.nodes.find((n) => n.id === "step:1")!)).toEqual(NODE_SIZE.stepCollapsed);
    expect(nodeSize(flow.nodes.find((n) => n.id === "step:2")!)).toEqual(NODE_SIZE.step);
  });
});
