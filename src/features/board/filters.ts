import type { Task } from "@/lib/bindings";
import type { BoardFilters } from "./types";

export const defaultFilters: BoardFilters = {
  needsYou: false,
  priorities: [],
  showHidden: false,
  archived: false,
  colors: [],
  jira: null,
  projects: [],
};

export function isNeedsYou(task: Pick<Task, "state">): boolean {
  return task.state.type === "NeedsYou";
}

function matchesJira(task: Task, jira: BoardFilters["jira"]): boolean {
  if (jira === null) return true;
  if ("key" in jira) return task.jira_key?.toLowerCase().startsWith(jira.key.toLowerCase()) ?? false;
  return (task.jira_key !== null) === jira.linked;
}

/** Archived, hidden, needs-you, priority, colour, Jira and project, applied in that order. */
export function applyFilters(tasks: Task[], filters: BoardFilters): Task[] {
  return tasks.filter(
    (t) =>
      (filters.archived ? t.archived : !t.archived) &&
      (filters.showHidden || !t.hidden) &&
      (!filters.needsYou || isNeedsYou(t)) &&
      (filters.priorities.length === 0 || filters.priorities.includes(t.priority)) &&
      (filters.colors.length === 0 || (t.color !== null && filters.colors.includes(t.color))) &&
      matchesJira(t, filters.jira) &&
      (filters.projects.length === 0 || filters.projects.includes(t.project_id)),
  );
}

/** The filter button's badge. `projects` is not counted: it is the Mixed chip row, not the filter popover. */
export function countActiveFilters(filters: BoardFilters): number {
  return (
    Number(filters.needsYou) +
    Number(filters.showHidden) +
    Number(filters.archived) +
    Number(filters.priorities.length > 0) +
    Number(filters.colors.length > 0) +
    Number(filters.jira !== null)
  );
}
