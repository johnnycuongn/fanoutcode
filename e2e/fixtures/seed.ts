// Pure data for the smoke suite, importable from Node (the spec) and the browser (the IPC mock): one project, three
// tasks, resolved settings, and the canvas's sample event log and plan for the Build task. Every value is checked
// with `satisfies` against the generated bindings, so a shape change in another plan fails `npm run typecheck`.
import type {
  CostSummary,
  PlanDoc,
  Project,
  ResolvedSettings,
  Task,
  TaskEvent,
  WindowInfo,
} from "../../src/lib/bindings";
import { MEMORY_DIR, TASK_ID, sampleBuild, samplePlan } from "../../src/features/canvas/fixtures";

export const STAGES = ["Spec", "Plan", "Build", "Review", "Test", "Commit", "Done"] as const;
const NOW = "2026-10-24T05:00:00Z";

export const PROJECT = {
  id: "p-0a1",
  name: "app",
  root: "/repo/app",
  color: "indigo",
  memory_location: { type: "AppStorage" },
  created_at: NOW,
} satisfies Project;

function task(over: Pick<Task, "id" | "title" | "stage" | "state"> & Partial<Task>): Task {
  return {
    project_id: PROJECT.id,
    workspace_id: null,
    priority: "None",
    color: null,
    flagged: false,
    hidden: false,
    archived: false,
    jira_key: null,
    agent_kind: "ClaudeCode",
    worktree: "Off",
    worktree_path: null,
    branch: null,
    memory_dir: `/mem/${over.id}`,
    primary_session: null,
    claude_session_id: null,
    linked_roots: [],
    base: [],
    created_at: NOW,
    updated_at: NOW,
    stage_entered_at: NOW,
    state_entered_at: NOW,
    links_bound_at: null,
    ...over,
  } satisfies Task;
}

export const BUILD_TASK = task({
  id: TASK_ID,
  title: "Canvas toolbar",
  stage: "Build",
  state: { type: "Working" },
  memory_dir: MEMORY_DIR,
});
export const NEEDS_YOU_TASK = task({
  id: "t-1d2",
  title: "Rate limiter",
  stage: "Plan",
  state: { type: "NeedsYou", reason: { type: "StageDone" } },
});
export const IDLE_TASK = task({ id: "t-2e3", title: "Fix login copy", stage: "Spec", state: { type: "Idle" } });
export const TASKS: Task[] = [BUILD_TASK, NEEDS_YOU_TASK, IDLE_TASK];

export const SETTINGS = {
  theme: "Light",
  accent: "Blue",
  detail_level: "Medium",
  dock_position: "Bottom",
  team_view_arrangement: "HubGraph",
  team_view_placement: "DockGrows",
  team_canvas_layout: "AutoReflow",
  team_drill_in_lens: "ByFile",
  workspace_project_layer: "MultiRoot",
  workspace_board: "Swimlanes",
  worktree: "Off",
  worktree_convention: "FanoutCodeManaged",
  worktrees_dir: null,
  may_commit: false,
  test_command: null,
  test_enabled: false,
  advance_on_green: false,
  livespec_level: "Off",
  agent_kind: "ClaudeCode",
  model_provider: "ClaudeCli",
  cost_overview: false,
  terminal_theme: { type: "FollowApp" },
  terminal_font_family: "JetBrains Mono",
  terminal_font_size: 13,
  terminal_ligatures: false,
  scrollback_lines: 10000,
  jira: null,
  jira_transitions: { enabled: false, on_stage: {} },
  first_run_complete: true,
} satisfies ResolvedSettings;

export const WINDOW = {
  window_id: "w-1",
  title: PROJECT.name,
  container: { type: "Project", id: PROJECT.id },
  project_ids: [PROJECT.id],
} satisfies WindowInfo;

export const EVENTS: Record<string, TaskEvent[]> = { [TASK_ID]: sampleBuild() };

/** The sample plan as Plan 05's `PlanDoc` would parse it. */
export const PLANS: Record<string, PlanDoc> = {
  [TASK_ID]: {
    steps: samplePlan().map((s, i, all) => ({
      number: s.number,
      title: s.title,
      done: s.done,
      deps: s.after.length > 0 ? s.after : i > 0 ? [all[i - 1].number] : [],
      explicit_deps: s.after.length > 0,
      repo: null,
    })),
    team: [],
  },
};

export const EMPTY_PLAN = { steps: [], team: [] } satisfies PlanDoc;

export const COST = {
  cost_usd: 0,
  input_tokens: 0,
  output_tokens: 0,
  estimated: false,
  live: null,
  by_project: [],
  by_task: [],
  by_stage: [],
  by_day: [],
  by_model: [],
  by_agent: [],
} satisfies CostSummary;
