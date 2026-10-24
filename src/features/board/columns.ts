import type { AgentState, Priority, Stage, Task } from "@/lib/bindings";
import type { BoardColumn } from "./types";

export const STAGES: readonly Stage[] = ["Spec", "Plan", "Build", "Review", "Test", "Commit", "Done"];

export function stageIndex(stage: Stage): number {
  return STAGES.indexOf(stage);
}

export function isForward(from: Stage, to: Stage): boolean {
  return stageIndex(to) > stageIndex(from);
}

export function stateRank(state: AgentState): 0 | 1 | 2 {
  return state.type === "NeedsYou" ? 0 : state.type === "Working" ? 1 : 2;
}

const PRIORITY_RANK: Record<Priority, number> = { High: 0, Medium: 1, Low: 2, None: 3 };

export function priorityRank(priority: Priority): number {
  return PRIORITY_RANK[priority];
}

function time(iso: string): number {
  const ms = Date.parse(iso);
  return Number.isNaN(ms) ? 0 : ms;
}

/**
 * Needs-you first, then working, then idle; flagged before unflagged; High to None; newest update first; id last so
 * the order is stable. The contracts give `Task` no manual order, so order is derived and never dragged.
 */
export function orderWithinColumn(tasks: Task[]): Task[] {
  return [...tasks].sort(
    (a, b) =>
      stateRank(a.state) - stateRank(b.state) ||
      Number(b.flagged) - Number(a.flagged) ||
      priorityRank(a.priority) - priorityRank(b.priority) ||
      time(b.updated_at) - time(a.updated_at) ||
      a.id.localeCompare(b.id),
  );
}

/** Always seven columns in pipeline order; empty ones are kept. */
export function groupByStage(tasks: Task[]): BoardColumn[] {
  return STAGES.map((stage) => {
    const inStage = orderWithinColumn(tasks.filter((t) => t.stage === stage));
    return { stage, tasks: inStage, count: inStage.length };
  });
}

/** Every lane present in `laneOrder`, empty ones kept; tasks of unknown projects are dropped. */
export function groupByLane(tasks: Task[], laneOrder: string[]): Map<string, Task[]> {
  const lanes = new Map<string, Task[]>(laneOrder.map((id) => [id, []]));
  for (const task of tasks) lanes.get(task.project_id)?.push(task);
  return lanes;
}
