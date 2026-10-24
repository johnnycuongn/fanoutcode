// STUB(plan 07): the smallest Board with Plan 07's exact props (A5, A20); Plan 07 replaces this file.
import { createElement, type ReactElement } from "react";
import { EmptyState } from "@/components/ui";
import type { Container, DetailLevel, Project, ProjectLayer, WorkspaceBoard } from "@/lib/bindings";

export type BoardSettings = {
  detailLevel: DetailLevel;
  detailLevelByProject: Record<string, DetailLevel>;
  costOverview: boolean;
  workspaceBoard: WorkspaceBoard;
  projectLayer: ProjectLayer;
};

export type OpenTarget = { kind: "current" } | { kind: "new_view" } | { kind: "window"; window_id: string };

export type BoardProps = {
  container: Container;
  projects: Project[];
  settings: BoardSettings;
  focusedProject: string | null;
  currentWindowId: string;
  onOpenTask(taskId: string, target: OpenTarget): void;
  onNewTask(): void;
};

export function Board(props: BoardProps): ReactElement {
  return createElement(EmptyState, { title: "Board", description: `${props.projects.length} project(s)` });
}
