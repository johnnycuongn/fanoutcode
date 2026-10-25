// Test builders for the generated domain types. Every enum value in a test comes from here, the layout helpers
// (src/features/views/layout.ts) or projectContainer/workspaceContainer (src/lib/stores/tasksStore.ts).
import type { AgentState, CostSummary, HandbackReason, Project, Task, View, Workspace } from "@/lib/bindings";

const AT = "2026-10-24T08:00:00Z";

export const WORKING: AgentState = { type: "Working" };
export const IDLE: AgentState = { type: "Idle" };
export function needsYou(reason: HandbackReason = { type: "StageDone" }): AgentState {
  return { type: "NeedsYou", reason };
}

export function makeProject(over: Partial<Project> = {}): Project {
  return {
    id: "proj-a1",
    name: "Alpha",
    root: "/r/alpha",
    color: "indigo",
    memory_location: { type: "AppStorage" },
    created_at: AT,
    ...over,
  };
}

export function makeTask(over: Partial<Task> = {}): Task {
  return {
    id: "task-b2",
    project_id: "proj-a1",
    workspace_id: null,
    title: "A task",
    stage: "Spec",
    state: IDLE,
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
    memory_dir: "/data/memory/2026-10-24-a-task",
    primary_session: null,
    claude_session_id: null,
    linked_roots: [],
    base: [],
    created_at: AT,
    updated_at: AT,
    stage_entered_at: AT,
    state_entered_at: AT,
    links_bound_at: null,
    ...over,
  };
}

export function makeView(over: Partial<View> = {}): View {
  return {
    id: "view-c3",
    container: { type: "Project", id: "proj-a1" },
    name: "View 1",
    order: 0,
    layout: { type: "Leaf", pane: "pane-aa", content: { type: "Empty" } },
    ...over,
  };
}

export function makeWorkspace(over: Partial<Workspace> = {}): Workspace {
  return {
    id: "ws-d4",
    name: "Alpha workspace",
    file_path: "/data/Alpha workspace.teamcode-workspace.json",
    dirty: false,
    projects: [{ project_id: "proj-a1", order: 0, protected: false }],
    policy: "ReadAndModifyOnRequest",
    ...over,
  };
}

export function makeCostSummary(over: Partial<CostSummary> = {}): CostSummary {
  return {
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
    ...over,
  };
}
