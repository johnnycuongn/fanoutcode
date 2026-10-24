// Cost on the board: how a summary reads (`~$0.42` while estimated) and, from Task 17, the cost store.
import type { CostSummary } from "./stubs/plan05";

/** Two decimals; a `~` prefix marks an estimate. */
export function formatCost(c: Pick<CostSummary, "cost_usd" | "estimated">): string {
  return `${c.estimated ? "~" : ""}$${c.cost_usd.toFixed(2)}`;
}

/** `999`, `12.3k`, `3.4M`. */
export function formatTokens(n: number): string {
  if (n < 1000) return String(n);
  const [value, unit] = n < 1_000_000 ? [n / 1000, "k"] : [n / 1_000_000, "M"];
  return `${value.toFixed(1).replace(/\.0$/, "")}${unit}`;
}
