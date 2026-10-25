// The header's cost text: "$0.42 · 12.3k tokens", "~" when any part is estimated (§10.6).
import type { CostSummary } from "@/lib/bindings";
import { formatUsd } from "@/lib/format";

function tokens(n: number): string {
  if (n < 1000) return String(n);
  return n < 1_000_000 ? `${(n / 1000).toFixed(1)}k` : `${(n / 1_000_000).toFixed(1)}M`;
}

export function formatCost(c: Pick<CostSummary, "cost_usd" | "input_tokens" | "output_tokens" | "estimated">): string {
  return `${formatUsd(c.cost_usd, c.estimated)} · ${tokens(c.input_tokens + c.output_tokens)} tokens`;
}
