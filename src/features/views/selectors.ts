// Pure selectors over a view: the tasks it shows, their home projects and how many need you.
import type { ProjectId, Task, TaskId, View } from "@/lib/bindings";
import { leaves } from "./layout";

/** Distinct task ids shown in the view's Task and TeamCanvas panes. */
export function viewTaskIds(view: View): TaskId[] {
  const ids: TaskId[] = [];
  for (const { content } of leaves(view.layout))
    if ((content.type === "Task" || content.type === "TeamCanvas") && !ids.includes(content.task_id))
      ids.push(content.task_id);
  return ids;
}

/** Distinct needs-you tasks among the view's Task panes (the tab's amber count). */
export function tabNeedsYouCount(view: View, tasksById: Record<TaskId, Task>): number {
  const ids = new Set<TaskId>();
  for (const { content } of leaves(view.layout))
    if (content.type === "Task" && tasksById[content.task_id]?.state.type === "NeedsYou") ids.add(content.task_id);
  return ids.size;
}

/** The home projects of the view's tasks (and terminals), in first-seen order, for the tab's colour dots. */
export function viewProjectIds(view: View, tasksById: Record<TaskId, Task>): ProjectId[] {
  const ids: ProjectId[] = [];
  for (const { content } of leaves(view.layout)) {
    const id =
      content.type === "Terminal"
        ? content.project_id
        : content.type === "Empty"
          ? undefined
          : tasksById[content.task_id]?.project_id;
    if (id && !ids.includes(id)) ids.push(id);
  }
  return ids;
}
