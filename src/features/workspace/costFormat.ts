// Formatting and grouping for the Costs panel; the numbers are Plan 05's `CostSummary` (A6, A19).
import type { CostLine, CostSummary } from "@/lib/bindings";

export type CostGroup = "project" | "task" | "stage" | "day";

/** `$1.23`, `~$1.23` for an estimate, `<$0.01` under a cent. */
export function formatUsd(n: number, estimated: boolean): string {
  const prefix = estimated ? "~" : "";
  if (n > 0 && n < 0.01) return `${prefix}<$0.01`;
  return `${prefix}$${n.toFixed(2)}`;
}

/** `950`, `12.3k`, `1.2M`. */
export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

const FIELD = { project: "by_project", task: "by_task", stage: "by_stage", day: "by_day" } as const;

/** The breakdown for `group`: by cost, most first; days newest first. */
export function rowsFor(summary: CostSummary, group: CostGroup): CostLine[] {
  const rows = [...summary[FIELD[group]]];
  return group === "day"
    ? rows.sort((a, b) => b.key.localeCompare(a.key))
    : rows.sort((a, b) => b.cost_usd - a.cost_usd);
}
