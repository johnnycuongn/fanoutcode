// The words the overlap UI uses (§7): a row's relation label, the other task's title, a row key and an overlap count.
import type { CommandKind, OverlapRow, Task, TaskId } from "@/lib/bindings";

// Keyed by string so a relation added in Rust before this map learns it still shows, under its raw name.
const RELATION_LABELS: Record<string, string> = {
  Claimed: "claimed",
  GrantedOnTop: "granted on top",
  TakenOver: "taken over",
  TakenOverBy: "taken over by",
  PlannedBy: "planned by",
  Committed: "committed",
};

/** The row's relation chip text: "holding" while the edit is held, else the relation's label. */
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

/** "1 overlap", "N overlaps". */
export function overlapsText(n: number): string {
  return n === 1 ? "1 overlap" : `${n} overlaps`;
}

/** The distinct relation labels of the rows on `path` in `repoRoot`, for a file row's chips. */
export function fileRelationLabels(rows: readonly OverlapRow[], repoRoot: string, path: string): string[] {
  const labels = rows.filter((r) => r.repo_root === repoRoot && r.path === path).map(relationLabel);
  return [...new Set(labels)];
}

/** Whether a shell command stashes (`git stash` in any form but `list` and `show`), read from its text. */
export function isStash(command: string): boolean {
  return /(^|[\s;&|(])git\s+(?:-\S+\s+(?:\S+\s+)?)*stash(?!\s+(?:list|show)\b)(?=\s|$|[;&|)])/.test(command);
}

/** The Proceed sub-line for a stopped command: what it does to the other task's uncommitted changes (§5.6). */
export function commandEffect(command: string, kind: CommandKind, other: string, paths: readonly string[]): string {
  const files = paths.join(", ");
  const what =
    kind === "Commits"
      ? "go into this task's commit"
      : isStash(command)
        ? "are stashed and leave the working tree"
        : "are discarded from the working tree";
  return `Runs \`${command}\`: ${other}'s uncommitted changes in ${files} ${what}.`;
}
