// The team canvas as its own pane (Own pane placement; PaneContent::TeamCanvas, A5 and A20).
import type { ReactElement } from "react";
import type { TaskId } from "@/lib/bindings";
import { TeamTab } from "./TeamTab";

export function CanvasPane({ taskId }: { taskId: TaskId }): ReactElement {
  return <TeamTab taskId={taskId} variant="pane" />;
}
