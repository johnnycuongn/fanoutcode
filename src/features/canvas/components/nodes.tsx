// React Flow wrappers: each card between a top target handle and a bottom source handle. Node `data` is the card's
// data plus the view options, so a card renders the same inside React Flow and in a plain render test.
import { Handle, Position, type Node, type NodeProps, type NodeTypes } from "@xyflow/react";
import { animate } from "motion";
import { useEffect, useRef, type ReactElement } from "react";
import { useReducedMotion } from "@/lib/motion";
import type { AgentNodeData, FileNodeData, StepNodeData, ViewOptions } from "../types";
import { AgentHub } from "./cards/AgentHub";
import { FileCard } from "./cards/FileCard";
import { StepCard } from "./cards/StepCard";

type WithView<T> = T & { view: ViewOptions };
export type StepFlowNode = Node<WithView<StepNodeData & { stackCount?: number }>, "step">;
export type AgentFlowNode = Node<WithView<AgentNodeData>, "agent">;
export type FileFlowNode = Node<WithView<FileNodeData & { ownerRoles: Record<string, string> }>, "file">;
export type FlowNode = StepFlowNode | AgentFlowNode | FileFlowNode;

const HIDDEN = { opacity: 0 };
const SIDES = [
  ["top", Position.Top],
  ["right", Position.Right],
  ["bottom", Position.Bottom],
  ["left", Position.Left],
] as const;

/** A source and a target handle on every side (`<side>-out`, `<side>-in`); edges pick the sides that face. */
function Handles({ children }: { children: ReactElement }): ReactElement {
  return (
    <>
      {SIDES.map(([side, position]) => (
        <Handle
          key={`${side}-in`}
          id={`${side}-in`}
          type="target"
          position={position}
          isConnectable={false}
          style={HIDDEN}
        />
      ))}
      {children}
      {SIDES.map(([side, position]) => (
        <Handle
          key={`${side}-out`}
          id={`${side}-out`}
          type="source"
          position={position}
          isConnectable={false}
          style={HIDDEN}
        />
      ))}
    </>
  );
}

export function StepNode({ data }: NodeProps<StepFlowNode>): ReactElement {
  return (
    <Handles>
      <StepCard data={data} view={data.view} />
    </Handles>
  );
}

export function AgentNode({ data }: NodeProps<AgentFlowNode>): ReactElement {
  return (
    <Handles>
      <AgentHub data={data} view={data.view} />
    </Handles>
  );
}

/** A file that just became shared pulses its warning border once (§11.3); never under reduced motion. */
export function FileNode({ data }: NodeProps<FileFlowNode>): ReactElement {
  const reduced = useReducedMotion();
  const el = useRef<HTMLDivElement>(null);
  const prev = useRef(data.sharedAt);
  useEffect(() => {
    const was = prev.current;
    prev.current = data.sharedAt;
    if (was === null && data.sharedAt !== null && !reduced && el.current)
      animate(el.current, { scale: [1, 1.04, 1] }, { duration: 0.6 });
  }, [data.sharedAt, reduced]);
  return (
    <Handles>
      <div ref={el} data-pulse-target="">
        <FileCard data={data} view={data.view} />
      </div>
    </Handles>
  );
}

export const nodeTypes = { step: StepNode, agent: AgentNode, file: FileNode } satisfies NodeTypes;
