// "Bring <base> in" from a Done worktree task (Plan 11 §6.3): a task with no base yet confirms one first in
// BaseBranchDialog, then the integrate prompt is injected; errors show like any other action's.
import { useState, type ReactNode } from "react";
import type { Task } from "@/lib/bindings";
import { BaseBranchDialog } from "@/features/overlap";
import { taskPageStore } from "../store";
import { tasksSource } from "../taskSource";

export function useBringBaseIn(task: Task | undefined): { start(): void; dialog: ReactNode } {
  const [asking, setAsking] = useState(false);
  const start = () => {
    if (!task) return;
    if (task.base_branch === null) setAsking(true);
    else void taskPageStore.integrate(task.id);
  };
  const dialog = task ? (
    <BaseBranchDialog
      task={task}
      open={asking}
      onClose={() => setAsking(false)}
      onConfirmed={(stored) => {
        tasksSource.put(stored);
        setAsking(false);
        void taskPageStore.integrate(task.id);
      }}
    />
  ) : null;
  return { start, dialog };
}
