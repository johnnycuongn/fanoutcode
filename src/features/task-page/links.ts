// §4.1: a session's linked roots are fixed when it starts. The header hints when the window's other project roots no
// longer match the ones the running session was given.
import type { Project, Task } from "@/lib/bindings";

export function linksChanged(task: Task, projects: Project[]): boolean {
  if (task.primary_session === null) return false;
  const current = projects
    .filter((p) => p.id !== task.project_id)
    .map((p) => p.root)
    .sort();
  const bound = [...task.linked_roots].sort();
  return current.length !== bound.length || current.some((root, i) => root !== bound[i]);
}
