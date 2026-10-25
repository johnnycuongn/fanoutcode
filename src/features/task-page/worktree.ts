// The header menu's worktree switch: flips the task's worktree choice through `tasks_update` (before a session).
import { toast } from "@/components/ui";
import type { Task } from "@/lib/bindings";
import { live } from "@/lib/ipcResult";
import { tasksSource } from "./taskSource";

const { tasksUpdate } = live("tasksUpdate");

export function toggleWorktree(task: Pick<Task, "id" | "worktree">): void {
  void tasksUpdate(task.id, {
    priority: null,
    color: null,
    clear_color: false,
    flagged: null,
    hidden: null,
    jira_key: null,
    clear_jira_key: false,
    worktree: task.worktree === "On" ? "Off" : "On",
    agent_kind: null,
  }).then(tasksSource.put, (e) => toast.error(String(e)));
}
