// Plan 08's test builders, shared by the task-page, dock, changes and spec-editor tests (feature boundaries forbid
// one feature's tests importing another's internals, so they live here; task-page/testing/fixtures.ts re-exports
// them under the plan's path).
import type { Task } from "@/lib/bindings";

const AT = "2026-10-24T05:12:00Z";

/** A task in Plan, idle, with a live session and one linked root. */
export function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    project_id: "proj-1",
    workspace_id: null,
    title: "Add the dock",
    stage: "Plan",
    state: { type: "Idle" },
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
    memory_dir: "/m/2026-10-24-a",
    primary_session: "sess-1",
    claude_session_id: null,
    linked_roots: ["/b"],
    base: [],
    created_at: AT,
    updated_at: AT,
    stage_entered_at: AT,
    state_entered_at: AT,
    links_bound_at: null,
    ...overrides,
  };
}
