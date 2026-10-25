// The task-page and pane wiring of A5: the Team tab for Plan 08's `TaskPageProps.renderTeam`, the Own-pane opener for
// `onOpenTeamPane`, and the `PaneContent::TeamCanvas` value. `@/features/views` is imported only at call time, so the
// views ↔ canvas cycle never runs at module load.
import type { PaneContent, TaskId } from "@/lib/bindings";
import type { TaskPageProps } from "@/features/task-page";
import { TeamTab } from "./components/TeamTab";

/** The only canvas code that spells the `PaneContent::TeamCanvas` wire shape. */
export function teamCanvasContent(taskId: TaskId): PaneContent {
  return { type: "TeamCanvas", task_id: taskId };
}

export const renderTeam: NonNullable<TaskPageProps["renderTeam"]> = (task) => (
  <TeamTab taskId={task.id} variant="dock" />
);

/** Side by side (A5's `Horizontal`); Plan 06 focuses a pane already showing the canvas instead of splitting again. */
export const onOpenTeamPane: NonNullable<TaskPageProps["onOpenTeamPane"]> = (task) => {
  void import("@/features/views").then((views) => views.splitFocusedPane("Horizontal", teamCanvasContent(task.id)));
};
