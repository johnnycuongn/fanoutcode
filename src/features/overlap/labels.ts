// The words the overlap UI uses: a row's relation as a chip label (§7) and an overlap count.
import type { OverlapRow } from "@/lib/bindings";

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
export function relationLabel(row: OverlapRow): string {
  if (row.held) return "holding";
  return RELATION_LABELS[row.relation] ?? row.relation;
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
