// Every task's overlaps (§7): the rows `overlap_list` returns for a task the developer has opened, and the counts
// `overlap-changed` reports for any task, so tabs and cards stay current without loading rows.
import { commands, events, type Decision, type OverlapRow, type Task, type TaskId, type View } from "@/lib/bindings";
import { unwrap } from "@/lib/ipcResult";
import { createStore } from "@/lib/stores/createStore";
import { viewTaskIds } from "@/features/views";
import type { OverlapState } from "./types";

export const overlapStore = createStore<OverlapState>({ byTask: {}, counts: {} });

/** The rows waiting on the developer. */
export function pendingRows(rows: OverlapRow[]): OverlapRow[] {
  return rows.filter((r) => r.pending !== null);
}

function setCounts(taskId: TaskId, count: number, pending: number): void {
  const now = overlapStore.get().counts[taskId];
  if (now?.count === count && now.pending === pending) return;
  overlapStore.set((s) => ({ ...s, counts: { ...s.counts, [taskId]: { count, pending } } }));
}

/** Loads the task's rows and recomputes its counts from them; a failed call leaves the state as it was. */
export async function refreshOverlaps(taskId: TaskId): Promise<void> {
  const r = await commands.overlapList(taskId);
  if (r?.status !== "ok") return;
  overlapStore.set((s) => ({ ...s, byTask: { ...s.byTask, [taskId]: r.data } }));
  setCounts(taskId, r.data.length, pendingRows(r.data).length);
}

/** Subscribes `overlap-changed`: the counts always, the rows again when the task's rows are loaded. */
export function startOverlapEvents(): () => void {
  const off = events.overlapChanged.listen((e) => {
    const { task_id, count, pending } = e.payload;
    setCounts(task_id, count, pending);
    if (overlapStore.get().byTask[task_id]) void refreshOverlaps(task_id);
  });
  return () => void off.then((fn) => fn());
}

/** Decides the row's pending overlap, then refreshes; rejects with the command's error string. */
export async function decide(taskId: TaskId, row: OverlapRow, decision: Decision): Promise<void> {
  unwrap(await commands.overlapDecide(taskId, row.repo_root, row.path, row.other_task, decision));
  await refreshOverlaps(taskId);
}

export function overlapCount(s: OverlapState, taskId: TaskId): number {
  return s.counts[taskId]?.count ?? 0;
}

/** The overlaps of the view's distinct loaded tasks, summed (the view tab's tooltip). */
export function viewOverlapCount(view: View, tasksById: Record<TaskId, Task>, s: OverlapState): number {
  return viewTaskIds(view)
    .filter((id) => tasksById[id])
    .reduce((n, id) => n + overlapCount(s, id), 0);
}
