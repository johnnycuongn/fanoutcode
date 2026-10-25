// The one file in this plan that knows Plan 06's tasksStore API (A10): the task page reads and writes Task records
// only through this adapter, so tasksStore stays the only copy.
import type { Task, TaskId } from "@/lib/bindings";
import { tasksStore } from "@/lib/stores/tasksStore";

export interface TaskSource {
  get(taskId: TaskId): Task | undefined;
  put(task: Task): void;
  subscribe(fn: () => void): () => void;
}

export const tasksSource: TaskSource = {
  get: (taskId) => tasksStore.get().byId[taskId],
  // An upsert of the record only: the board's order is tasksStore's own business (loadTasks, task-changed).
  put: (task) => tasksStore.set((s) => (s.byId[task.id] === task ? s : { ...s, byId: { ...s.byId, [task.id]: task } })),
  subscribe: (fn) => tasksStore.subscribe(fn),
};
