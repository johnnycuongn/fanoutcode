// Cost fixtures for the costs tests: a full A6 `CostSummary` and an A19 `CostLine`.
import type { CostLine, CostSummary } from "@/lib/bindings";

export const line = (key: string, cost: number): CostLine => ({
  key,
  cost_usd: cost,
  input_tokens: 0,
  output_tokens: 0,
  estimated: false,
});
export function summary(over: Partial<CostSummary>): CostSummary {
  return {
    cost_usd: 0,
    input_tokens: 0,
    output_tokens: 0,
    estimated: false,
    live: null,
    by_project: [],
    by_task: [],
    by_stage: [],
    by_day: [],
    by_model: [],
    by_agent: [],
    ...over,
  };
}
