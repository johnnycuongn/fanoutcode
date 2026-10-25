// Edge renderers: `touches` (an agent at its step or file, a flowing dash while the agent works) and `depends`
// (the flow canvas's grey "depends on" edges).
import { BaseEdge, getBezierPath, type Edge, type EdgeProps, type EdgeTypes } from "@xyflow/react";
import type { ReactElement } from "react";
import { cx } from "@/components/ui";

export type EdgeData = { working: boolean };
export type TouchesFlowEdge = Edge<EdgeData, "touches">;
export type DependsFlowEdge = Edge<EdgeData, "depends">;
export type FlowEdge = TouchesFlowEdge | DependsFlowEdge;

function pathOf(p: EdgeProps<FlowEdge>): string {
  return getBezierPath({
    sourceX: p.sourceX,
    sourceY: p.sourceY,
    sourcePosition: p.sourcePosition,
    targetX: p.targetX,
    targetY: p.targetY,
    targetPosition: p.targetPosition,
  })[0];
}

export function TouchesEdge(props: EdgeProps<TouchesFlowEdge>): ReactElement {
  return (
    <g className={cx("edge-touches text-accent", props.data?.working && "edge-flowing")}>
      <BaseEdge id={props.id} path={pathOf(props)} markerEnd={props.markerEnd} />
    </g>
  );
}

export function DependsEdge(props: EdgeProps<DependsFlowEdge>): ReactElement {
  return (
    <g className="edge-depends text-mute">
      <title>depends on</title>
      <BaseEdge id={props.id} path={pathOf(props)} markerEnd={props.markerEnd} />
    </g>
  );
}

export const edgeTypes = { touches: TouchesEdge, depends: DependsEdge } satisfies EdgeTypes;
