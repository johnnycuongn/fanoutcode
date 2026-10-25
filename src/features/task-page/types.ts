// The task page's state and command set. The Task itself stays in Plan 06's tasksStore (through taskSource.ts).
import type { CostSummary, DockPosition, Stage, StageAction, Task, TeamPlacement } from "@/lib/bindings";
import type { Live } from "@/lib/ipcResult";

/** The settings the page reads, from `settings_get` at the task's scope. */
export interface TaskSettings {
  mayCommit: boolean;
  costOverview: boolean;
  testCommand: string | null;
  advanceOnGreen: boolean;
  dockPosition: DockPosition;
  teamViewPlacement: TeamPlacement;
}

export type TaskPageIpc = Live<
  "tasksGet" | "tasksAction" | "tasksRename" | "sessionsStop" | "sessionsOpen" | "costSummary" | "settingsGet"
>;

/** Page-local only. */
export interface TaskPageState {
  seen: Record<string, Pick<Task, "stage" | "state">>;
  settings: Record<string, TaskSettings>;
  queued: Record<string, StageAction>;
  cost: Record<string, CostSummary>;
  contextPct: Record<string, number>;
  errors: Record<string, string>;
}

export type { Stage };
