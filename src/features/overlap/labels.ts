// Display text for overlap rows (§7): the relation label and the other task's title.
import type { OverlapRow, Task, TaskId } from "@/lib/bindings";

/**
 * Keyed by string, not by `Relation`, so a variant the bindings do not know yet (TakenOverBy) still has its label
 * and an unknown one falls back to its raw name.
 */
const RELATION_LABELS: Record<string, string> = {
  Claimed: "claimed",
  GrantedOnTop: "granted on top",
  TakenOver: "taken over",
  TakenOverBy: "taken over by",
  PlannedBy: "planned by",
  Committed: "committed",
};

export function relationLabel(row: Pick<OverlapRow, "relation" | "held">): string {
  if (row.held) return "holding";
  const relation: string = row.relation;
  return RELATION_LABELS[relation] ?? relation;
}

export const ANOTHER_SESSION = "another session";

/** The other task's title; "another session" when the claim has no task or the task is not loaded here. */
export function otherTitle(id: TaskId | null, byId: Record<TaskId, Task>): string {
  return (id && byId[id]?.title) || ANOTHER_SESSION;
}

/** A stable key for a row: the same file may overlap with two tasks. */
export function rowKey(row: Pick<OverlapRow, "repo_root" | "path" | "other_task">): string {
  return `${row.repo_root}\u0000${row.path}\u0000${row.other_task ?? ""}`;
}
