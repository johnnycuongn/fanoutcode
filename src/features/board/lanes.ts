// Swimlane order and which rows are collapsed, remembered per container in localStorage. A view preference, not data:
// reads and writes are guarded, and a missing, corrupt or blocked store just means the project order.
import type { Container } from "@/lib/bindings";
import type { LaneState } from "./types";

export type { LaneState };

export const laneStorageKey = (container: Container): string =>
  `teamcode.board.lanes.${container.type}.${container.id}`;

const strings = (value: unknown): string[] =>
  Array.isArray(value) ? value.filter((v): v is string => typeof v === "string") : [];

/** Keeps `prev`'s order for projects that still exist and appends new ones; collapsed rows of gone projects drop out. */
export function reconcileLanes(prev: LaneState, projectIds: string[]): LaneState {
  const known = prev.order.filter((id) => projectIds.includes(id));
  return {
    order: [...known, ...projectIds.filter((id) => !known.includes(id))],
    collapsed: prev.collapsed.filter((id) => projectIds.includes(id)),
  };
}

export function loadLaneState(key: string, projectIds: string[]): LaneState {
  let saved: LaneState = { order: [], collapsed: [] };
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(key) ?? "null");
    if (typeof parsed === "object" && parsed !== null) {
      const { order, collapsed } = parsed as Record<string, unknown>;
      saved = { order: strings(order), collapsed: strings(collapsed) };
    }
  } catch {
    // Missing, corrupt or blocked storage: fall back to the project order.
  }
  return reconcileLanes(saved, projectIds);
}

export function saveLaneState(key: string, state: LaneState): void {
  try {
    localStorage.setItem(key, JSON.stringify(state));
  } catch {
    // A view preference: losing it is harmless.
  }
}

/** A copy of `order` with the item at `from` moved to `to`; an index outside the list changes nothing. */
export function moveLane(order: string[], from: number, to: number): string[] {
  if (from < 0 || from >= order.length || to < 0 || to >= order.length) return [...order];
  const next = [...order];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved!);
  return next;
}
