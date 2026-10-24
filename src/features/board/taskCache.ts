// The board's only access to the shared task cache (`tasksStore`, owned by Plan 06). The board keeps no task copy of
// its own, so the sidebar and the board read the same tasks; every read and write goes through here, so a change to
// the cache's shape touches this one file.
import type { Task } from "@/lib/bindings";
import { tasksStore } from "@/lib/stores/tasksStore";

export type TaskMap = Record<string, Task>;

export const taskMap = (): TaskMap => tasksStore.get().byId;
export const subscribeTasks = tasksStore.subscribe;

/** Insert or replace one task; a new id is appended to the cache's order. */
export function putTask(task: Task): void {
  tasksStore.set((s) => ({
    ...s,
    byId: { ...s.byId, [task.id]: task },
    order: s.byId[task.id] ? s.order : [...s.order, task.id],
  }));
}

export function dropTask(id: string): void {
  tasksStore.set((s) => {
    if (!s.byId[id]) return s;
    const byId = { ...s.byId };
    delete byId[id];
    return { ...s, byId, order: s.order.filter((x) => x !== id) };
  });
}

/** Replace the cache with the tasks of the container the board is showing. */
export function replaceTasks(tasks: Task[]): void {
  tasksStore.set((s) => ({
    ...s,
    byId: Object.fromEntries(tasks.map((t) => [t.id, t])),
    order: tasks.map((t) => t.id),
    loaded: true,
  }));
}

export function resetTaskCache(): void {
  tasksStore.reset();
}
