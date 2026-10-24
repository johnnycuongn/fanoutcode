// A LayoutNode: a Split becomes a Resizable over its two children, a Leaf a PaneFrame.
import type { ReactElement } from "react";
import { Resizable } from "@/components/ui";
import type { LayoutNode, PaneId, ViewId } from "@/lib/bindings";
import { setPaneRatio } from "../store";
import type { LayoutPath } from "../types";
import { PaneFrame } from "./PaneFrame";

export type SplitNodeProps = {
  viewId: ViewId;
  node: LayoutNode;
  path: LayoutPath;
  focusedPane: PaneId;
  dragging: boolean;
};

export function SplitNode({ viewId, node, path, focusedPane, dragging }: SplitNodeProps): ReactElement {
  if (node.type === "Leaf")
    return (
      <PaneFrame
        viewId={viewId}
        pane={node.pane}
        content={node.content}
        focused={node.pane === focusedPane}
        dragging={dragging}
      />
    );
  const child = (n: LayoutNode, step: "first" | "second") => (
    <SplitNode viewId={viewId} node={n} path={[...path, step]} focusedPane={focusedPane} dragging={dragging} />
  );
  return (
    <Resizable
      direction={node.direction === "Horizontal" ? "horizontal" : "vertical"}
      ratio={node.ratio}
      onRatioChange={(r) => setPaneRatio(viewId, path, r)}
      handleLabel="Resize panes"
      first={child(node.first, "first")}
      second={child(node.second, "second")}
    />
  );
}
