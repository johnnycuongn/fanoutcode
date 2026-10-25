// What the task page's body shows, and whether the page should open the task's session (Plan 04: nothing else opens
// a task's primary session; `sessions_open` starts or resumes it and is idempotent).
import type { Task } from "@/lib/bindings";

export type BodyKind = "spec" | "terminal" | "no-session";

export function bodyFor(task: Task): BodyKind {
  if (task.stage === "Spec") return "spec";
  return task.primary_session ? "terminal" : "no-session";
}

export function wantsSession(task: Task): boolean {
  return !task.archived && task.stage !== "Spec" && task.stage !== "Done";
}
