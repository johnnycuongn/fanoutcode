// The new-task flow: `openNewTask()` (called by the shell's ⌘T, the palette and the New task button) opens the board's
// sheet; `createTask` calls `tasks_create`. If no Board is mounted when it is called, the sheet opens when one mounts.
import { toast } from "@/components/ui";
import { createStore } from "@/lib/stores/createStore";
import { ipc } from "./ipc";
import { boardStore, upsertTask } from "./store";

export const newTaskStore = createStore<{ open: boolean }>({ open: false });

export const openNewTask = (): void => newTaskStore.set({ open: true });
export const closeNewTask = (): void => newTaskStore.set({ open: false });

/** The project a new task goes to: the picked or focused project, else the first. Null on an empty window. */
export function defaultProject(): string | null {
  const s = boardStore.get();
  const container = s.container;
  if (container?.type === "Project") return container.id;
  return s.pickedProject ?? s.focusedProject ?? s.projectOrder[0] ?? null;
}

/** Creates the task; the card arrives with `task-changed`, and is also put in the cache so it shows at once. */
export async function createTask(projectId: string, intent: string, jiraKey: string | null): Promise<boolean> {
  const text = intent.trim();
  if (!text) return false;
  const container = boardStore.get().container;
  try {
    const task = await ipc.tasksCreate(projectId, text, jiraKey, container?.type === "Workspace" ? container.id : null);
    upsertTask(task);
    return true;
  } catch (err) {
    toast.error(err instanceof Error ? err.message : String(err));
    return false;
  }
}
