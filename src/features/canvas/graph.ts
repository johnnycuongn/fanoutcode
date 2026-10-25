// The two arrangements (§10.7): the hub graph (agents, steps and files, `touches` edges only) and the flow canvas
// (steps left to right by dependency, agents docked below pointing at their current cards, files inside the cards).
import type { TeamArrangement } from "@/lib/bindings";
import type { CanvasEdge, CanvasGraph, CanvasModel, CanvasNode } from "./types";

export const nodeId = {
  agent: (id: string) => "agent:" + id,
  step: (n: number) => "step:" + n,
  file: (path: string) => "file:" + path,
};

function edge(source: string, target: string, kind: CanvasEdge["kind"], working: boolean): CanvasEdge {
  return { id: source + "->" + target, source, target, kind, working };
}

export function toGraph(model: CanvasModel, arrangement: TeamArrangement): CanvasGraph {
  const hub = arrangement === "HubGraph";
  const nodes: CanvasNode[] = [
    ...model.agents.map((a): CanvasNode => ({ id: nodeId.agent(a.agentId), kind: "agent", data: a })),
    ...model.steps.map((s): CanvasNode => ({ id: nodeId.step(s.number), kind: "step", data: s })),
    ...(hub ? model.files.map((f): CanvasNode => ({ id: nodeId.file(f.path), kind: "file", data: f })) : []),
  ];
  const ids = new Set(nodes.map((n) => n.id));
  const edges: CanvasEdge[] = [];
  const add = (e: CanvasEdge) => {
    if (ids.has(e.source) && ids.has(e.target)) edges.push(e);
  };
  if (!hub) {
    model.steps.forEach((s, i) => {
      const from = s.after.length > 0 ? s.after : i > 0 && s.number > 1 ? [model.steps[i - 1].number] : [];
      for (const n of from) add(edge(nodeId.step(n), nodeId.step(s.number), "depends", false));
    });
  }
  for (const a of model.agents) {
    if (!hub && !a.working) continue;
    const source = nodeId.agent(a.agentId);
    for (const n of a.steps) add(edge(source, nodeId.step(n), "touches", a.working));
    if (hub) for (const f of a.files) add(edge(source, f, "touches", a.working));
  }
  return { nodes, edges };
}

/** Changes only when the set of nodes or a step's status changes, so a tool line or cost never re-lays out. */
export function layoutKey(graph: CanvasGraph): string {
  return graph.nodes
    .map((n) => (n.kind === "step" ? `${n.id}@${n.data.status}` : n.id))
    .sort()
    .join("|");
}
