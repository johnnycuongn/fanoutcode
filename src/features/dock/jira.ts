// STUB(plan 10): Plan 10's `jira_issue`, `jira_comment` and `jira_search` commands and its `Issue` type are not in
// src/lib/bindings.ts yet. Each wrapper looks its command up on the generated `commands` at call time, so it works
// unchanged once the binding lands (and in tests); until then it rejects (issue, comment) or finds nothing (search).
// When Plan 10 merges, delete this file and use `live("jiraIssue", "jiraComment", "jiraSearch")` and its `Issue`.
import { commands, type ProjectId, type Result, type Task, type TaskId, type TaskPatch } from "@/lib/bindings";
import { live, unwrap } from "@/lib/ipcResult";

/** STUB(plan 10): Plan 10's `jira::Issue` (the fields this tab reads are required, the rest optional). */
export type Issue = {
  key: string;
  summary: string;
  status: string;
  assignee: string | null;
  url: string;
  issue_type?: string;
  description?: string;
  updated?: string;
};

type Pending = {
  jiraIssue?(projectId: ProjectId, key: string): Promise<Result<Issue, string>>;
  jiraComment?(projectId: ProjectId, key: string, body: string): Promise<Result<null, string>>;
  jiraSearch?(projectId: ProjectId, query: string): Promise<Result<Issue[], string>>;
};

const pending = () => commands as unknown as Pending;
const NOT_YET = "Jira is not available yet.";

const { tasksUpdate } = live("tasksUpdate");

/** The full TaskPatch for a change to the Jira key alone (`null` unlinks). */
export function jiraKeyPatch(jiraKey: string | null): TaskPatch {
  return {
    priority: null,
    color: null,
    clear_color: false,
    flagged: null,
    hidden: null,
    jira_key: jiraKey,
    clear_jira_key: jiraKey === null,
    worktree: null,
    agent_kind: null,
  };
}

export const jiraIpc = {
  async jiraIssue(projectId: ProjectId, key: string): Promise<Issue> {
    const call = pending().jiraIssue;
    if (!call) throw NOT_YET;
    return unwrap(await call(projectId, key));
  },
  async jiraComment(projectId: ProjectId, key: string, body: string): Promise<void> {
    const call = pending().jiraComment;
    if (!call) throw NOT_YET;
    unwrap(await call(projectId, key, body));
  },
  async jiraSearch(projectId: ProjectId, query: string): Promise<Issue[]> {
    const call = pending().jiraSearch;
    return call ? unwrap(await call(projectId, query)) : [];
  },
  tasksUpdate: (id: TaskId, patch: { jira_key: string | null }): Promise<Task> =>
    tasksUpdate(id, jiraKeyPatch(patch.jira_key)),
};
