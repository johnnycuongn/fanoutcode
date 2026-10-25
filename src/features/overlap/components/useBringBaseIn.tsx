// "Bring <base> in" from a Done worktree task (§6.3), shared by the Overlap tab and the task page's overflow menu: a
// task with no base yet confirms one first in BaseBranchDialog, then the integrate prompt is injected; a rejected call
// shows as a toast unless the caller passes its own `integrate`.
import { useState, type ReactNode } from "react";
import { toast } from "@/components/ui";
import type { Task, TaskId } from "@/lib/bindings";
import { errorMessage, live } from "@/lib/ipcResult";
import { tasksStore } from "@/lib/stores/tasksStore";
import { BaseBranchDialog } from "./BaseBranchDialog";

const ipc = live("overlapIntegrate");

/** Whether the task can bring its base in: a Done, unarchived task working in a worktree. */
export function canBringBaseIn(task: Pick<Task, "archived" | "stage" | "worktree_path">): boolean {
  return !task.archived && task.stage === "Done" && task.worktree_path !== null;
}

/** "Bring main in", or "Bring base in" while the task has no base yet. */
export function bringBaseInLabel(task: Pick<Task, "base_branch">): string {
  return `Bring ${task.base_branch ?? "base"} in`;
}

async function integrateBase(taskId: TaskId): Promise<void> {
  try {
    await ipc.overlapIntegrate(taskId);
  } catch (e) {
    toast.error(errorMessage(e));
  }
}

function putTask(task: Task): void {
  tasksStore.set((s) => (s.byId[task.id] === task ? s : { ...s, byId: { ...s.byId, [task.id]: task } }));
}

export interface BringBaseIn {
  start(): void;
  dialog: ReactNode;
}

export function useBringBaseIn(
  task: Task | undefined,
  integrate: (taskId: TaskId) => Promise<void> = integrateBase,
): BringBaseIn {
  const [asking, setAsking] = useState(false);
  const start = () => {
    if (!task) return;
    if (task.base_branch === null) setAsking(true);
    else void integrate(task.id);
  };
  const dialog = task ? (
    <BaseBranchDialog
      task={task}
      open={asking}
      onClose={() => setAsking(false)}
      onConfirmed={(stored) => {
        putTask(stored);
        setAsking(false);
        void integrate(task.id);
      }}
    />
  ) : null;
  return { start, dialog };
}
