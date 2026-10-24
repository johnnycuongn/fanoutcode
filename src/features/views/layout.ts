// Pure edits of a view's LayoutNode tree, so a split, close or move responds at once; the store persists the
// result through `views_update_layout`. `Horizontal` = first and second side by side (⌘D), `Vertical` = stacked
// (⌘⇧D) (A5). The constructors here and in tasksStore.ts are the only code that builds these serde shapes (A17).
import type { LayoutNode, PaneContent, PaneId, ProjectId, SessionId, SplitDirection, TaskId } from "@/lib/bindings";
import type { Dir, Edge, LayoutPath, LeafInfo, Rect, SplitLayout } from "./types";

export const MIN_RATIO = 0.1;
export const MAX_RATIO = 0.9;
const EPS = 0.001;

export function leaf(pane: PaneId, content: PaneContent): LayoutNode {
  return { type: "Leaf", pane, content };
}

export function split(direction: SplitDirection, ratio: number, first: LayoutNode, second: LayoutNode): LayoutNode {
  return { type: "Split", direction, ratio, first, second };
}

export const EMPTY_PANE: PaneContent = { type: "Empty" };

export function taskContent(task_id: TaskId): PaneContent {
  return { type: "Task", task_id };
}

export function teamCanvasContent(task_id: TaskId): PaneContent {
  return { type: "TeamCanvas", task_id };
}

export function terminalContent(project_id: ProjectId, session_id: SessionId): PaneContent {
  return { type: "Terminal", project_id, session_id };
}

export function sameContent(a: PaneContent, b: PaneContent): boolean {
  switch (a.type) {
    case "Empty":
      return b.type === "Empty";
    case "Task":
    case "TeamCanvas":
      return b.type === a.type && b.task_id === a.task_id;
    case "Terminal":
      return b.type === "Terminal" && b.project_id === a.project_id && b.session_id === a.session_id;
  }
}

export function newPaneId(): PaneId {
  const bytes = crypto.getRandomValues(new Uint8Array(4));
  return "pane-" + [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
}

export function leaves(node: LayoutNode, path: LayoutPath = []): LeafInfo[] {
  if (node.type === "Leaf") return [{ pane: node.pane, content: node.content, path }];
  return [...leaves(node.first, [...path, "first"]), ...leaves(node.second, [...path, "second"])];
}

export function findPane(node: LayoutNode, pane: PaneId): LayoutPath | null {
  return leaves(node).find((l) => l.pane === pane)?.path ?? null;
}

export function nodeAt(node: LayoutNode, path: LayoutPath): LayoutNode {
  let at = node;
  for (const step of path) {
    if (at.type !== "Split") break;
    at = at[step];
  }
  return at;
}

/** Replaces the node at `path` with `fn(node)`; the rest of the tree is shared. */
function updateAt(node: LayoutNode, path: LayoutPath, fn: (n: LayoutNode) => LayoutNode): LayoutNode {
  if (path.length === 0) return fn(node);
  if (node.type !== "Split") return node;
  const [step, ...rest] = path;
  return { ...node, [step]: updateAt(node[step], rest, fn) };
}

export function splitPane(
  node: LayoutNode,
  pane: PaneId,
  direction: SplitDirection,
  newPane: PaneId,
  content: PaneContent = EMPTY_PANE,
): LayoutNode {
  const path = findPane(node, pane);
  if (!path) return node;
  return updateAt(node, path, (old) => split(direction, 0.5, old, leaf(newPane, content)));
}

/** The sibling takes the parent's place; removing the only leaf empties it instead (a view keeps one pane). */
export function removePane(node: LayoutNode, pane: PaneId): LayoutNode {
  const path = findPane(node, pane);
  if (!path) return node;
  if (path.length === 0) return leaf(pane, EMPTY_PANE);
  const parent = path.slice(0, -1);
  const last = path[path.length - 1];
  return updateAt(node, parent, (p) => (p.type === "Split" ? (last === "first" ? p.second : p.first) : p));
}

export function setRatio(node: LayoutNode, path: LayoutPath, ratio: number): LayoutNode {
  const r = Math.min(MAX_RATIO, Math.max(MIN_RATIO, ratio));
  return updateAt(node, path, (n) => (n.type === "Split" ? { ...n, ratio: r } : n));
}

export function setContent(node: LayoutNode, pane: PaneId, content: PaneContent): LayoutNode {
  const path = findPane(node, pane);
  if (!path) return node;
  return updateAt(node, path, (n) => (n.type === "Leaf" ? { ...n, content } : n));
}

/** Drops `pane` on `edge` of `target`: removed from its place, then `target` is split around it. */
export function movePane(node: LayoutNode, pane: PaneId, target: PaneId, edge: Edge): LayoutNode {
  if (pane === target) return node;
  const moving = leaves(node).find((l) => l.pane === pane);
  if (!moving || !findPane(node, target)) return node;
  const without = removePane(node, pane);
  const path = findPane(without, target);
  if (!path) return node;
  const direction: SplitDirection = edge === "left" || edge === "right" ? "Horizontal" : "Vertical";
  const moved = leaf(pane, moving.content);
  const before = edge === "left" || edge === "top";
  return updateAt(without, path, (t) => (before ? split(direction, 0.5, moved, t) : split(direction, 0.5, t, moved)));
}

export function rects(node: LayoutNode, at: Rect = { x: 0, y: 0, w: 1, h: 1 }, out = new Map<PaneId, Rect>()) {
  if (node.type === "Leaf") {
    out.set(node.pane, at);
    return out;
  }
  const s = node as SplitLayout;
  if (s.direction === "Horizontal") {
    const w1 = at.w * s.ratio;
    rects(s.first, { ...at, w: w1 }, out);
    rects(s.second, { ...at, x: at.x + w1, w: at.w - w1 }, out);
  } else {
    const h1 = at.h * s.ratio;
    rects(s.first, { ...at, h: h1 }, out);
    rects(s.second, { ...at, y: at.y + h1, h: at.h - h1 }, out);
  }
  return out;
}

const overlap = (a0: number, a1: number, b0: number, b1: number) => Math.max(0, Math.min(a1, b1) - Math.max(a0, b0));

/** The leaf next to `pane` in `dir`: adjacent within 0.001, the largest overlap on the other axis, then nearest start. */
export function neighbour(node: LayoutNode, pane: PaneId, dir: Dir): PaneId | null {
  const all = rects(node);
  const cur = all.get(pane);
  if (!cur) return null;
  let best: { id: PaneId; overlap: number; start: number } | null = null;
  for (const [id, r] of all) {
    if (id === pane) continue;
    const adjacent =
      dir === "right"
        ? Math.abs(r.x - (cur.x + cur.w)) < EPS
        : dir === "left"
          ? Math.abs(r.x + r.w - cur.x) < EPS
          : dir === "down"
            ? Math.abs(r.y - (cur.y + cur.h)) < EPS
            : Math.abs(r.y + r.h - cur.y) < EPS;
    if (!adjacent) continue;
    const horizontal = dir === "left" || dir === "right";
    const ov = horizontal
      ? overlap(cur.y, cur.y + cur.h, r.y, r.y + r.h)
      : overlap(cur.x, cur.x + cur.w, r.x, r.x + r.w);
    if (ov <= EPS) continue;
    const start = horizontal ? r.y : r.x;
    if (!best || ov > best.overlap + EPS || (Math.abs(ov - best.overlap) <= EPS && start < best.start))
      best = { id, overlap: ov, start };
  }
  return best?.id ?? null;
}
