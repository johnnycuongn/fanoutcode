// Positions from elkjs (bundled build, main thread): stress for the hub graph, layered left-to-right for the flow
// canvas with the agents docked along the bottom (§10.7 "Arrangement").
import type { ElkNode } from "elkjs/lib/elk-api";
import type { TeamArrangement } from "@/lib/bindings";
import { stackDone } from "./positions";
import type { CanvasGraph, CanvasNode, CanvasPosition } from "./types";

/** The bundled build runs on the main thread: layouts under 200 nodes finish inside a frame and tests run in Node. */
export const ELK_WORKER = false;

export const NODE_SIZE = {
  agent: { width: 160, height: 72 },
  step: { width: 240, height: 112 },
  stepCollapsed: { width: 240, height: 40 },
  file: { width: 200, height: 48 },
} as const;

const AGENT_GAP = 16;
const DOCK_GAP = 80;
const STACK_LEFT = 280;

type Elk = InstanceType<typeof import("elkjs/lib/elk.bundled.js").default>;
let elkLoad: Promise<Elk> | null = null;
/** elkjs (about 1.4 MB) loads on the first layout, so the Team tab renders before it arrives. */
function elk(): Promise<Elk> {
  elkLoad ??= import("elkjs/lib/elk.bundled.js").then((m) => new m.default());
  return elkLoad;
}

export function nodeSize(node: CanvasNode): { width: number; height: number } {
  if (node.kind === "step") return node.data.status === "active" ? NODE_SIZE.step : NODE_SIZE.stepCollapsed;
  return NODE_SIZE[node.kind];
}

async function run(
  nodes: CanvasNode[],
  edges: CanvasGraph["edges"],
  layoutOptions: Record<string, string>,
): Promise<CanvasPosition[]> {
  if (nodes.length === 0) return [];
  const ids = new Set(nodes.map((n) => n.id));
  const graph: ElkNode = {
    id: "root",
    layoutOptions,
    children: nodes.map((n) => ({ id: n.id, ...nodeSize(n) })),
    edges: edges
      .filter((e) => ids.has(e.source) && ids.has(e.target))
      .map((e) => ({ id: e.id, sources: [e.source], targets: [e.target] })),
  };
  const out = await (await elk()).layout(graph);
  return (out.children ?? []).map((c) => ({ node_id: c.id, x: c.x ?? 0, y: c.y ?? 0 }));
}

type Size = { width: number; height: number };

/**
 * Removes overlaps (with a 32 px gap) by pushing each overlapping pair apart along its shallower axis; anything still
 * overlapping after the passes drops into a row below, so the result never overlaps.
 */
export function separate(positions: CanvasPosition[], sizes: Map<string, Size>, gap = 32): CanvasPosition[] {
  const boxes = positions.map((p) => ({ ...p, ...(sizes.get(p.node_id) ?? { width: 0, height: 0 }) }));
  const overlap = (a: (typeof boxes)[number], b: (typeof boxes)[number]) => ({
    x: Math.min(a.x + a.width + gap - b.x, b.x + b.width + gap - a.x),
    y: Math.min(a.y + a.height + gap - b.y, b.y + b.height + gap - a.y),
  });
  for (let pass = 0; pass < 60; pass++) {
    let moved = false;
    for (let i = 0; i < boxes.length; i++) {
      for (let j = i + 1; j < boxes.length; j++) {
        const a = boxes[i];
        const b = boxes[j];
        const o = overlap(a, b);
        if (o.x <= 0 || o.y <= 0) continue;
        moved = true;
        if (o.x < o.y) {
          const dir = a.x + a.width / 2 <= b.x + b.width / 2 ? 1 : -1;
          a.x -= (dir * o.x) / 2;
          b.x += (dir * o.x) / 2;
        } else {
          const dir = a.y + a.height / 2 <= b.y + b.height / 2 ? 1 : -1;
          a.y -= (dir * o.y) / 2;
          b.y += (dir * o.y) / 2;
        }
      }
    }
    if (!moved) break;
  }
  let rowX = Math.min(0, ...boxes.map((b) => b.x));
  const rowY = Math.max(0, ...boxes.map((b) => b.y + b.height)) + gap;
  for (let i = 0; i < boxes.length; i++) {
    const hit = boxes.some((b, j) => j < i && overlap(b, boxes[i]).x > gap && overlap(b, boxes[i]).y > gap);
    if (!hit) continue;
    boxes[i].x = rowX;
    boxes[i].y = rowY;
    rowX += boxes[i].width + gap;
  }
  return boxes.map((b) => ({ node_id: b.node_id, x: Math.round(b.x), y: Math.round(b.y) }));
}

/** Agents sit 80 px below the lowest step, under the mean x of the steps they point at, at least a card apart. */
export function dockAgents(stepPositions: CanvasPosition[], graph: CanvasGraph): CanvasPosition[] {
  const steps = new Map(graph.nodes.filter((n) => n.kind === "step").map((n) => [n.id, n]));
  const placed = new Map(stepPositions.map((p) => [p.node_id, p]));
  let bottom = 0;
  for (const p of stepPositions) {
    const n = steps.get(p.node_id);
    if (n) bottom = Math.max(bottom, p.y + nodeSize(n).height);
  }
  const y = bottom + DOCK_GAP;
  const pitch = NODE_SIZE.agent.width + AGENT_GAP;
  const agents = graph.nodes.filter((n) => n.kind === "agent");
  const xs = new Map<string, number>();
  for (const a of agents) {
    const targets = graph.edges
      .filter((e) => e.kind === "touches" && e.source === a.id)
      .map((e) => placed.get(e.target))
      .filter((p): p is CanvasPosition => p !== undefined);
    if (targets.length > 0) xs.set(a.id, targets.reduce((sum, p) => sum + p.x, 0) / targets.length);
  }
  let next = xs.size > 0 ? Math.max(...xs.values()) + pitch : Math.min(0, ...stepPositions.map((p) => p.x));
  for (const a of agents) {
    if (xs.has(a.id)) continue;
    xs.set(a.id, next);
    next += pitch;
  }
  const sorted = agents.map((a) => ({ node_id: a.id, x: xs.get(a.id) ?? 0, y })).sort((a, b) => a.x - b.x);
  for (let i = 1; i < sorted.length; i++) sorted[i].x = Math.max(sorted[i].x, sorted[i - 1].x + pitch);
  return sorted;
}

export async function layoutGraph(
  graph: CanvasGraph,
  arrangement: TeamArrangement,
  opts: { stackDone: boolean },
): Promise<CanvasPosition[]> {
  if (arrangement === "Flow") {
    const steps = graph.nodes.filter((n) => n.kind === "step");
    const stepPositions = await run(
      steps,
      graph.edges.filter((e) => e.kind === "depends"),
      {
        "elk.algorithm": "layered",
        "elk.direction": "RIGHT",
        "elk.layered.spacing.nodeNodeBetweenLayers": "64",
        "elk.spacing.nodeNode": "24",
      },
    );
    return [...stepPositions, ...dockAgents(stepPositions, graph)];
  }
  const isDone = (n: CanvasNode) => opts.stackDone && n.kind === "step" && n.data.status === "done";
  const hubNodes = graph.nodes.filter((n) => !isDone(n));
  const stressed = await run(hubNodes, graph.edges, {
    "elk.algorithm": "org.eclipse.elk.stress",
    "org.eclipse.elk.stress.desiredEdgeLength": "140",
    "elk.spacing.nodeNode": "32",
  });
  // Stress places centres by edge length only, so cards can overlap; `separate` pushes them apart afterwards.
  const laidOut = separate(stressed, new Map(hubNodes.map((n) => [n.id, nodeSize(n)])));
  const doneIds = graph.nodes.filter(isDone).map((n) => n.id);
  if (doneIds.length === 0) return laidOut;
  const anchor =
    laidOut.length === 0
      ? { x: 0, y: 0 }
      : { x: Math.min(...laidOut.map((p) => p.x)) - STACK_LEFT, y: Math.min(...laidOut.map((p) => p.y)) };
  return [
    ...laidOut,
    ...stackDone(
      doneIds.map((node_id) => ({ node_id, x: 0, y: 0 })),
      doneIds,
      anchor,
    ),
  ];
}
