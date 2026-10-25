// The canvas surface (§10.7): React Flow over the store's graph with pan, zoom, minimap and drag; positions spring
// to each layout, a drag persists through moveNode, and clicking an agent or a file opens the drill-in panel.
import "@xyflow/react/dist/style.css";
import "../canvas.css";
import {
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type NodeChange,
  type NodeMouseHandler,
  type OnNodeDrag,
} from "@xyflow/react";
import { useEffect, useMemo, useRef, useState, type ReactElement } from "react";
import { useReducedMotion } from "@/lib/motion";
import { repoFor, showRepoBadges } from "../repo";
import { moveNode, select, useCanvas, type CanvasState } from "../store";
import { NODE_SIZE } from "../layout";
import type { CanvasEdge, CanvasPosition, ViewOptions } from "../types";
import { edgeTypes, type FlowEdge } from "./edges";
import { nodeTypes, type FlowNode } from "./nodes";
import { usePositionSpring } from "./usePositionSpring";

type FlowState = Pick<CanvasState, "model" | "graph" | "settings" | "task" | "roots">;
type Side = "top" | "right" | "bottom" | "left";

/** The facing sides of two nodes: depends edges run left to right; others follow the dominant axis between centres. */
export function handlesFor(
  edge: Pick<CanvasEdge, "kind">,
  from: CanvasPosition | undefined,
  to: CanvasPosition | undefined,
): { sourceHandle: string; targetHandle: string } {
  let out: Side = "bottom";
  let into: Side = "top";
  if (edge.kind === "depends") [out, into] = ["right", "left"];
  else if (from && to) {
    const dx = to.x - from.x;
    const dy = to.y - from.y;
    if (Math.abs(dy) >= Math.abs(dx) - NODE_SIZE.step.width / 2)
      [out, into] = dy >= 0 ? ["bottom", "top"] : ["top", "bottom"];
    else [out, into] = dx >= 0 ? ["right", "left"] : ["left", "right"];
  }
  return { sourceHandle: `${out}-out`, targetHandle: `${into}-in` };
}

/** Graph nodes and edges as React Flow elements: view options, owner roles, the done stack, the drawn positions. */
export function toFlowElements(
  state: FlowState,
  positions: CanvasPosition[],
  reduced: boolean,
): { nodes: FlowNode[]; edges: FlowEdge[] } {
  const ownerRoles = Object.fromEntries(state.model.agents.map((a) => [a.agentId, a.role]));
  const view: ViewOptions = {
    detail: state.settings.detail,
    showCost: state.settings.showCost,
    showRepo: state.task ? showRepoBadges(state.task, state.roots) : false,
    roots: state.roots,
    reduced,
  };
  const at = new Map(positions.map((p) => [p.node_id, p]));
  const doneIds = state.graph.nodes.filter((n) => n.kind === "step" && n.data.status === "done").map((n) => n.id);
  const stacking = state.settings.layout === "AutoReflow" && state.settings.arrangement !== "Flow";
  const nodes = state.graph.nodes.map((n): FlowNode => {
    const p = at.get(n.id);
    const base = { id: n.id, position: p ? { x: p.x, y: p.y } : { x: 0, y: 0 }, hidden: positions.length > 0 && !p };
    if (n.kind === "step") {
      const i = doneIds.indexOf(n.id);
      const stackIndex = stacking && i >= 0 ? i : null;
      return {
        ...base,
        type: "step",
        zIndex: stackIndex === null ? 0 : doneIds.length - stackIndex,
        data: { ...n.data, stackIndex, stackCount: doneIds.length, view },
      };
    }
    if (n.kind === "agent") return { ...base, type: "agent", data: { ...n.data, view } };
    const repo = n.data.repo ?? repoFor(n.data.path, state.roots)?.root ?? null;
    return { ...base, type: "file", data: { ...n.data, repo, ownerRoles, view } };
  });
  const edges = state.graph.edges.map((e): FlowEdge => ({
    id: e.id,
    source: e.source,
    target: e.target,
    ...handlesFor(e, at.get(e.source), at.get(e.target)),
    type: e.kind,
    data: { working: e.working },
  }));
  return { nodes, edges };
}

function Surface({ taskId }: { taskId: string }): ReactElement {
  const s = useCanvas(taskId);
  const reduced = useReducedMotion();
  const rf = useReactFlow();
  const [drag, setDrag] = useState<ReadonlyMap<string, { x: number; y: number }>>(() => new Map());
  const dragging = useMemo(() => new Set(drag.keys()), [drag]);
  const targets = useMemo(() => s.positions.map((p) => ({ ...p, ...drag.get(p.node_id) })), [s.positions, drag]);
  const shown = usePositionSpring(targets, dragging);
  const { nodes, edges } = useMemo(() => toFlowElements(s, shown, reduced), [s, shown, reduced]);

  // Refit on the active steps and working agents when a layout changes that set (not on every frame or pan).
  const focus = useMemo(
    () =>
      s.graph.nodes
        .filter((n) => (n.kind === "step" && n.data.status === "active") || (n.kind === "agent" && n.data.working))
        .map((n) => n.id),
    [s.graph],
  );
  const fitted = useRef<string | null>(null);
  useEffect(() => {
    const key = focus.join("|");
    if (s.positions.length === 0 || focus.length === 0 || fitted.current === key) return;
    fitted.current = key;
    void rf.fitView({ nodes: focus.map((id) => ({ id })), padding: 0.3, maxZoom: 1, duration: reduced ? 150 : 300 });
  }, [focus, s.positions, rf, reduced]);

  const onNodesChange = (changes: NodeChange<FlowNode>[]) => {
    const moved = changes.filter((c) => c.type === "position" && c.dragging && c.position);
    if (moved.length === 0) return;
    setDrag((prev) => {
      const next = new Map(prev);
      for (const c of moved) if (c.type === "position" && c.position) next.set(c.id, c.position);
      return next;
    });
  };
  const onNodeDragStop: OnNodeDrag<FlowNode> = (_, node) => {
    moveNode(taskId, node.id, node.position.x, node.position.y);
    setDrag((prev) => {
      const next = new Map(prev);
      next.delete(node.id);
      return next;
    });
  };
  const onNodeClick: NodeMouseHandler<FlowNode> = (_, node) => {
    if (node.type === "agent") select(taskId, { kind: "agent", id: node.data.agentId });
    else if (node.type === "file") select(taskId, { kind: "file", id: node.id });
  };

  return (
    <ReactFlow<FlowNode, FlowEdge>
      className="bg-page"
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      onNodesChange={onNodesChange}
      onNodeDragStop={onNodeDragStop}
      onNodeClick={onNodeClick}
      nodesDraggable
      nodesConnectable={false}
      panOnScroll
      zoomOnPinch
      minZoom={0.25}
      maxZoom={2}
      fitView
      fitViewOptions={{ maxZoom: 1 }}
      proOptions={{ hideAttribution: true }}
    >
      <MiniMap pannable zoomable nodeClassName="fill-line" className="border border-line bg-card" />
    </ReactFlow>
  );
}

export function TeamCanvas({ taskId }: { taskId: string }): ReactElement {
  return (
    <div className="h-full w-full" data-team-canvas={taskId}>
      <ReactFlowProvider>
        <Surface taskId={taskId} />
      </ReactFlowProvider>
    </div>
  );
}
