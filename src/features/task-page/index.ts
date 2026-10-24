// STUB(plan 08): the smallest TaskPage with Plan 08's exact props and handle (A5); Plan 08 replaces this file.
import { createElement, useImperativeHandle, type ReactElement, type ReactNode, type Ref } from "react";
import { Panel } from "@/components/ui";
import type { Container, PaneId, Project, Task, TaskId } from "@/lib/bindings";

export interface TaskPageHandle {
  primaryAction(): void;
  rename(): void;
  archive(): void;
  toggleDock(): void;
  growTeam(): void;
  toggleTerminalStrip(): void;
  focusTerminal(): void;
}

export interface TaskPageProps {
  taskId: TaskId;
  paneId: PaneId;
  container: Container;
  projects: Project[];
  renderTeam?: (task: Task) => ReactNode;
  onBack(): void;
  onOpenTeamPane?(task: Task): void;
  ref?: Ref<TaskPageHandle>;
}

const noop = () => {};

export function TaskPage({ taskId, ref }: TaskPageProps): ReactElement {
  useImperativeHandle(ref, () => ({
    primaryAction: noop,
    rename: noop,
    archive: noop,
    toggleDock: noop,
    growTeam: noop,
    toggleTerminalStrip: noop,
    focusTerminal: noop,
  }));
  return createElement(Panel, { variant: "plain", className: "h-full text-mute" }, `Task ${taskId}`);
}
