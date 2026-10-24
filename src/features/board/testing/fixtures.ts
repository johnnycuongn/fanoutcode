// Builders for the board's tests. The only place a `Container` or a `Task` literal is spelled out.
import type { AgentState, Container, HandbackReason, Project, Task } from "@/lib/bindings";
import type { CostSummary, HookEvent, TaskEvent } from "../stubs/plan05";

export const WORKING: AgentState = { type: "Working" };
export const IDLE: AgentState = { type: "Idle" };

export function needsYou(reason: HandbackReason): AgentState {
  return { type: "NeedsYou", reason };
}

export function projectContainer(id: string): Container {
  return { type: "Project", id };
}

export function workspaceContainer(id: string): Container {
  return { type: "Workspace", id };
}

const T0 = "2026-10-24T10:00:00Z";

export function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "t-1",
    project_id: "p-1",
    workspace_id: null,
    title: "Task",
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
    memory_dir: "/tmp/memory/t-1",
    primary_session: null,
    claude_session_id: null,
    linked_roots: [],
    base: [],
    created_at: T0,
    updated_at: T0,
    stage_entered_at: T0,
    state_entered_at: T0,
    links_bound_at: null,
    ...overrides,
  };
}

export function makeProject(overrides: Partial<Project> = {}): Project {
  return {
    id: "p-1",
    name: "Project",
    root: "/repo",
    color: "indigo",
    memory_location: { type: "AppStorage" },
    created_at: T0,
    ...overrides,
  };
}

export function makeEvent(taskId: string, event: HookEvent, seq = 1): TaskEvent {
  return { seq, at: T0, task_id: taskId, agent_id: null, event };
}

export function makeCost(overrides: Partial<CostSummary> = {}): CostSummary {
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
    ...overrides,
  };
}
