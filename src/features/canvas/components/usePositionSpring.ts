// Node moves on the canvas (§11.3): a 300 ms spring, or a 150 ms ease under reduced motion. A tiny external store
// owns the in-flight positions so animation frames re-render through useSyncExternalStore; a node seen for the first
// time and a node being dragged take their target at once.
import { animate } from "motion";
import { useEffect, useState, useSyncExternalStore } from "react";
import { useReducedMotion } from "@/lib/motion";
import type { CanvasPosition } from "../types";

export function motionFor(
  reduced: boolean,
): { type: "spring"; duration: number; bounce: number } | { duration: number; ease: "easeOut" } {
  return reduced ? { duration: 0.15, ease: "easeOut" } : { type: "spring", duration: 0.3, bounce: 0.2 };
}

type Point = { x: number; y: number };

function createSprings() {
  let shown = new Map<string, Point>();
  let running: { stop(): void } | null = null;
  const listeners = new Set<() => void>();
  const emit = () => listeners.forEach((l) => l());
  return {
    subscribe(l: () => void) {
      listeners.add(l);
      return () => void listeners.delete(l);
    },
    get: () => shown,
    stop: () => running?.stop(),
    setTargets(targets: CanvasPosition[], dragging: ReadonlySet<string>, reduced: boolean) {
      const next = new Map<string, Point>();
      const moving: { id: string; from: Point; to: Point }[] = [];
      for (const t of targets) {
        const cur = shown.get(t.node_id);
        const to = { x: t.x, y: t.y };
        if (cur && !dragging.has(t.node_id) && (cur.x !== to.x || cur.y !== to.y)) {
          moving.push({ id: t.node_id, from: cur, to });
          next.set(t.node_id, cur);
        } else next.set(t.node_id, to);
      }
      const same =
        moving.length === 0 &&
        next.size === shown.size &&
        [...next].every(([id, p]) => shown.get(id)?.x === p.x && shown.get(id)?.y === p.y);
      if (same) return;
      running?.stop();
      running = null;
      shown = next;
      emit();
      if (moving.length === 0) return;
      running = animate(0, 1, {
        ...motionFor(reduced),
        onUpdate: (v) => {
          const frame = new Map(shown);
          for (const m of moving)
            frame.set(m.id, { x: m.from.x + (m.to.x - m.from.x) * v, y: m.from.y + (m.to.y - m.from.y) * v });
          shown = frame;
          emit();
        },
      });
    },
  };
}

/** The positions to draw this frame: each moved node springs from where it is to its target. */
export function usePositionSpring(targets: CanvasPosition[], dragging: ReadonlySet<string>): CanvasPosition[] {
  const reduced = useReducedMotion();
  const [springs] = useState(createSprings);
  const shown = useSyncExternalStore(springs.subscribe, springs.get, springs.get);
  useEffect(() => springs.setTargets(targets, dragging, reduced), [springs, targets, dragging, reduced]);
  useEffect(() => () => springs.stop(), [springs]);
  return targets.map((t) => {
    const p = shown.get(t.node_id);
    return p ? { node_id: t.node_id, x: p.x, y: p.y } : t;
  });
}
