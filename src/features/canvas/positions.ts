// Auto re-flow and Pinned (§10.7 "Live layout"): which laid-out positions replace the ones on screen.
import type { CanvasLayout, Stage } from "@/lib/bindings";
import type { CanvasPosition } from "./types";

export type LayoutTrigger = "initial" | "build-start" | "tidy" | "change";

export const STACK_OFFSET = 8;

export function layoutTrigger(prevStage: Stage | null, stage: Stage, hasSaved: boolean): LayoutTrigger {
  if (stage === "Build" && prevStage !== null && prevStage !== "Build") return "build-start";
  return hasSaved ? "change" : "initial";
}

/** Auto re-flow always takes the new layout; Pinned takes it on initial, build-start and tidy, else only for new ids. */
export function resolvePositions(args: {
  mode: CanvasLayout;
  trigger: LayoutTrigger;
  previous: CanvasPosition[];
  laidOut: CanvasPosition[];
  nodeIds: string[];
}): CanvasPosition[] {
  const { mode, trigger, previous, laidOut, nodeIds } = args;
  const keep = mode === "Pinned" && trigger === "change" ? new Map(previous.map((p) => [p.node_id, p])) : null;
  const fresh = new Map(laidOut.map((p) => [p.node_id, p]));
  const out: CanvasPosition[] = [];
  for (const id of nodeIds) {
    const p = keep?.get(id) ?? fresh.get(id);
    if (p) out.push(p);
  }
  return out;
}

export function prunePositions(positions: CanvasPosition[], nodeIds: string[]): CanvasPosition[] {
  const ids = new Set(nodeIds);
  return positions.filter((p) => ids.has(p.node_id));
}

/** The i-th done id moves to `anchor + 8·i` on both axes; other positions are untouched. */
export function stackDone(
  positions: CanvasPosition[],
  doneIds: string[],
  anchor: { x: number; y: number },
): CanvasPosition[] {
  const index = new Map(doneIds.map((id, i) => [id, i]));
  return positions.map((p) => {
    const i = index.get(p.node_id);
    return i === undefined ? p : { node_id: p.node_id, x: anchor.x + STACK_OFFSET * i, y: anchor.y + STACK_OFFSET * i };
  });
}
