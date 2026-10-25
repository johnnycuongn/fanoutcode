// Pure selectors over a view: the tasks it shows, their home projects, how many need you, and its status and stage.
import type { AgentStatus } from "@/components/ui";
import type { ProjectId, Stage, Task, TaskId, View } from "@/lib/bindings";
import { leaves } from "./layout";

/** Distinct task ids shown in the view's Task and TeamCanvas panes. */
export function viewTaskIds(view: View): TaskId[] {
  const ids: TaskId[] = [];
  for (const { content } of leaves(view.layout))
    if ((content.type === "Task" || content.type === "TeamCanvas") && !ids.includes(content.task_id))
      ids.push(content.task_id);
  return ids;
}

/** Distinct loaded tasks among the view's Task panes. */
function paneTasks(view: View, tasksById: Record<TaskId, Task>): Task[] {
  const tasks = new Map<TaskId, Task>();
  for (const { content } of leaves(view.layout)) {
    const task = content.type === "Task" ? tasksById[content.task_id] : undefined;
    if (task) tasks.set(task.id, task);
  }
  return [...tasks.values()];
}

/** Distinct needs-you tasks among the view's Task panes (the tab's amber count). */
export function tabNeedsYouCount(view: View, tasksById: Record<TaskId, Task>): number {
  return paneTasks(view, tasksById).filter((t) => t.state.type === "NeedsYou").length;
}

/** The view's aggregate state for the tab's beating dot: needs-you beats working beats idle; null with no task. */
export function viewStatus(view: View, tasksById: Record<TaskId, Task>): AgentStatus | null {
  const tasks = paneTasks(view, tasksById);
  if (tasks.length === 0) return null;
  if (tasks.some((t) => t.state.type === "NeedsYou")) return "needs-you";
  return tasks.some((t) => t.state.type === "Working") ? "working" : "idle";
}

/** The stage the view's Task-pane tasks share, "many" when they sit in more than one, null with no task. */
export function viewStage(view: View, tasksById: Record<TaskId, Task>): Stage | "many" | null {
  const stages = new Set(paneTasks(view, tasksById).map((t) => t.stage));
  if (stages.size === 0) return null;
  return stages.size === 1 ? [...stages][0] : "many";
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
