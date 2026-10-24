// Board-local types. Everything that crosses IPC comes from `@/lib/bindings` (or, until Plan 05 lands, the
// `stubs/plan05.ts` wrapper); these are view-model shapes only.
import type { DetailLevel, Priority, ProjectLayer, Stage, Task, WorkspaceBoard, WorktreeChoice } from "@/lib/bindings";

export type ChipTone = "working" | "needs_you" | "ok" | "idle";
export type CardChipModel = { tone: ChipTone; text: string; detail?: string };

export type JiraFilter = null | { linked: boolean } | { key: string };

/** Empty arrays mean "all"; `archived: true` shows only archived cards. */
export type BoardFilters = {
  needsYou: boolean;
  priorities: Priority[];
  showHidden: boolean;
  archived: boolean;
  colors: string[];
  jira: JiraFilter;
  projects: string[];
};

export type BoardColumn = { stage: Stage; tasks: Task[]; count: number };

export type Swimlane = {
  projectId: string;
  name: string;
  color: string;
  collapsed: boolean;
  columns: BoardColumn[];
  needsYou: number;
};

export type BoardMode = "single" | "swimlanes" | "one_project" | "mixed";

export type BoardSettings = {
  detailLevel: DetailLevel;
  detailLevelByProject: Record<string, DetailLevel>;
  costOverview: boolean;
  workspaceBoard: WorkspaceBoard;
  projectLayer: ProjectLayer;
};

export type OpenTarget = { kind: "current" } | { kind: "new_view" } | { kind: "window"; window_id: string };

/** `agentId` is null for a row seeded from `tasks_activity`, which carries no agent id. */
export type TeamMember = { agentId: string | null; role: string; working: boolean };

export type CardActivity = {
  lastLine: string | null;
  currentFile: { path: string; added: number; removed: number } | null;
  team: TeamMember[];
};

export type DropOutcome =
  | { kind: "move"; taskId: string; from: Stage; to: Stage }
  | { kind: "none" }
  | { kind: "rejected"; reason: "cross_lane" | "read_only" | "same_column" };

export type ConfirmOptions = { title: string; body?: string; confirmLabel: string; danger?: boolean };

/** What the board may change on a task. `null` clears the colour or the Jira key; a missing field is left alone. */
export type TaskPatchInput = {
  priority?: Priority;
  color?: string | null;
  flagged?: boolean;
  hidden?: boolean;
  jira_key?: string | null;
  worktree?: WorktreeChoice;
};

/** Swimlane order and which lanes are collapsed, persisted per container (lanes.ts). */
export type LaneState = { order: string[]; collapsed: string[] };
