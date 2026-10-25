// Commands and types this plan calls that other plans own and that are not in src/lib/bindings.ts yet. Each wrapper
// looks its command up on the generated `commands` object, so it works unchanged once the binding lands (and in
// tests, where the bindings mock supplies every name); until then it answers with an empty result. When the owning
// plan merges, delete the wrapper and the STUB type, and import the generated ones instead.
import { commands, type Result, type Scope, type TaskId } from "@/lib/bindings";

// STUB(plan 05): Plan 05's `cost::CostLine`, `LiveCost` and `CostSummary` (A6, A19).
export type CostLine = {
  key: string;
  cost_usd: number;
  input_tokens: number;
  output_tokens: number;
  estimated: boolean;
};
export type LiveCost = { cost_usd: number; context_used_pct: number; at: string };
export type CostSummary = {
  cost_usd: number;
  input_tokens: number;
  output_tokens: number;
  estimated: boolean;
  live: LiveCost | null;
  by_project: CostLine[];
  by_task: CostLine[];
  by_stage: CostLine[];
  by_day: CostLine[];
  by_model: CostLine[];
  by_agent: CostLine[];
};

// STUB(plan 05): Plan 05's `changes::ChangedFile` and `RepoChanges`, the fields this plan reads.
export type ChangedFile = { path: string; added: number; removed: number };
export type RepoChanges = { repo_root: string; files: ChangedFile[] };

type Pending = {
  costSummary?(scope: Scope): Promise<Result<CostSummary, string>>;
  gitChanges?(taskId: TaskId): Promise<Result<RepoChanges[], string>>;
};

const pending = () => commands as unknown as Pending;
const missing = <T>(name: string): Result<T, string> => ({ status: "error", error: `${name} is not available yet` });

/** STUB(plan 05): `cost_summary(scope) -> CostSummary`. */
export function costSummary(scope: Scope): Promise<Result<CostSummary, string>> {
  return pending().costSummary?.(scope) ?? Promise.resolve(missing("cost_summary"));
}

/** STUB(plan 05): `git_changes(task_id) -> Vec<RepoChanges>`. */
export function gitChanges(taskId: TaskId): Promise<Result<RepoChanges[], string>> {
  return pending().gitChanges?.(taskId) ?? Promise.resolve({ status: "ok", data: [] });
}
