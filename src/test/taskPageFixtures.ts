// Plan 08's test builders, shared by the task-page, dock, changes and spec-editor tests (feature boundaries forbid
// one feature's tests importing another's internals, so they live here; task-page/testing/fixtures.ts re-exports
// them under the plan's path).
import type { Task } from "@/lib/bindings";
import type { TaskSettings, TaskSource } from "@/features/task-page";

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
    base_branch: null,
    ...overrides,
  };
}

/** An in-memory TaskSource (stands in for Plan 06's tasksStore). */
export function makeTaskSource(tasks: Task[] = []): TaskSource {
  const byId = new Map(tasks.map((t) => [t.id, t]));
  const listeners = new Set<() => void>();
  return {
    get: (id) => byId.get(id),
    put(task) {
      byId.set(task.id, task);
      for (const l of [...listeners]) l();
    },
    subscribe(fn) {
      listeners.add(fn);
      return () => {
        listeners.delete(fn);
      };
    },
  };
}

/** Every flag off, no test command, the dock at the bottom and DockGrows. */
export function makeSettings(overrides: Partial<TaskSettings> = {}): TaskSettings {
  return {
    mayCommit: false,
    costOverview: false,
    testCommand: null,
    advanceOnGreen: false,
    dockPosition: "Bottom",
    teamViewPlacement: "DockGrows",
    ...overrides,
  };
}

/** An `events` stand-in: every `listen` resolves to a no-op unsubscribe. */
export const fakeEvents: Record<string, { listen: (cb: unknown) => Promise<() => void> }> = new Proxy(
  {},
  { get: () => ({ listen: async () => () => {} }) },
);
