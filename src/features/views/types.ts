// The views feature's types. `LayoutNode` and `PaneContent` are the generated shapes (A1, A17).
import type { LayoutNode, PaneContent, PaneId } from "@/lib/bindings";

/** From the root to a node; each step picks a Split's `first` or `second`. */
export type LayoutPath = Array<"first" | "second">;
export type Edge = "left" | "right" | "top" | "bottom";
export type Dir = "left" | "up" | "right" | "down";
/** In 0..1 units of the view. */
export type Rect = { x: number; y: number; w: number; h: number };
export type SplitLayout = Extract<LayoutNode, { type: "Split" }>;
export type LeafLayout = Extract<LayoutNode, { type: "Leaf" }>;
export type LeafInfo = { pane: PaneId; content: PaneContent; path: LayoutPath };
