// STUB(plan 09): the smallest CanvasPane with Plan 09's canonical props (A20); Plan 09 replaces this file and may add
// `renderTeam`, which the views feature passes to the task page when present.
import { createElement, type ReactElement } from "react";
import { Panel } from "@/components/ui";
import type { TaskId } from "@/lib/bindings";

export function CanvasPane({ taskId }: { taskId: TaskId }): ReactElement {
  return createElement(Panel, { variant: "plain", className: "h-full text-mute" }, `Team ${taskId}`);
}
