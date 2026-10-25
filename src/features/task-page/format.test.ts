import { expect, test } from "vitest";
import type { CostSummary } from "@/lib/bindings";
import { formatCost } from "./format";

test.each([
  [{ cost_usd: 0.4211, input_tokens: 10200, output_tokens: 2100, estimated: false }, "$0.42 · 12.3k tokens"],
  [{ cost_usd: 0.4211, input_tokens: 10200, output_tokens: 2100, estimated: true }, "~$0.42 · 12.3k tokens"],
  [{ cost_usd: 12.5, input_tokens: 1_000_000, output_tokens: 234_567, estimated: false }, "$12.50 · 1.2M tokens"],
  [{ cost_usd: 0, input_tokens: 0, output_tokens: 0, estimated: false }, "$0.00 · 0 tokens"],
])("formatCost(%o)", (c, out) => expect(formatCost(c as CostSummary)).toBe(out));
