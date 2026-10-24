// Cost on the board: how a summary reads (`~$0.42` while estimated) and the cost store, fed by `cost_summary` and kept
// current by `cost-updated`. Nothing here runs unless the cost overview is on.
import type { Container, Scope } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { ipc } from "./ipc";
import { track } from "./listen";
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

export const scopeFor = (container: Container): Scope =>
  container.type === "Workspace"
    ? { workspace: container.id, project: null, task: null }
    : { workspace: null, project: container.id, task: null };

export const scopeForTask = (taskId: string): Scope => ({ workspace: null, project: null, task: taskId });

export type CostState = { board: CostSummary | null; byTask: Record<string, CostSummary> };

export const costStore = createStore<CostState>({ board: null, byTask: {} });

let boardContainer: Container | null = null;
const tracked = new Set<string>();

export function resetCostStore(): void {
  boardContainer = null;
  tracked.clear();
  costStore.reset();
}

/** The board's total (its non-archived tasks, §10.6). A failed call keeps the last numbers. */
export async function loadBoardCost(container: Container): Promise<void> {
  boardContainer = container;
  try {
    const board = await ipc.costSummary(scopeFor(container));
    if (boardContainer === container) costStore.set({ board });
  } catch {
    // The numbers are a convenience; the board works without them.
  }
}

async function fetchTaskCost(taskId: string): Promise<void> {
  try {
    const summary = await ipc.costSummary(scopeForTask(taskId));
    if (tracked.has(taskId)) costStore.set((s) => ({ ...s, byTask: { ...s.byTask, [taskId]: summary } }));
  } catch {
    // As above.
  }
}

/** Fetches a task's cost once and keeps it current; called for cards that show or tooltip a cost. */
export function trackTaskCost(taskId: string): void {
  if (tracked.has(taskId)) return;
  tracked.add(taskId);
  void fetchTaskCost(taskId);
}

export function untrackTaskCost(taskId: string): void {
  if (!tracked.delete(taskId)) return;
  costStore.set((s) => {
    const byTask = { ...s.byTask };
    delete byTask[taskId];
    return { ...s, byTask };
  });
}

/** `cost-updated` refetches the task if a card tracks it, and always the board total. */
export function startCostEvents(): () => void {
  return track(
    ipc.on("cost-updated", ({ task_id }) => {
      if (tracked.has(task_id)) void fetchTaskCost(task_id);
      if (boardContainer) void loadBoardCost(boardContainer);
    }),
  );
}

export const useTaskCost = (taskId: string): CostSummary | null =>
  costStore.useSelector((s) => s.byTask[taskId] ?? null);
export const useBoardCost = (): CostSummary | null => costStore.useSelector((s) => s.board);
