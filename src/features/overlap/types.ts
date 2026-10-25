// The overlap feature's types: the per-task rows `overlap_list` returns and the counts `overlap-changed` reports.
import type { OverlapRow, TaskId } from "@/lib/bindings";

export type OverlapCounts = { count: number; pending: number };

export type OverlapState = {
  /** The rows of every task whose overlaps were loaded; a task absent here was never refreshed. */
  byTask: Record<TaskId, OverlapRow[]>;
  /** The latest counts per task, from a refresh or the event; drives the dock tab, cards and view tabs. */
  counts: Record<TaskId, OverlapCounts>;
};
