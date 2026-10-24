// Commands and types this plan calls that other plans own and that are not in src/lib/bindings.ts yet. Each wrapper
// looks its command up on the generated `commands` object, so it works unchanged once the binding lands (and in
// tests, where the bindings mock supplies every name); until then it answers with an empty result. When the owning
// plan merges, delete the wrapper and the STUB type, and import the generated ones instead.
import { commands, type ProjectId, type Result, type Scope, type SessionId, type TaskId } from "@/lib/bindings";
import type { WorkspaceId, Workspace } from "@/lib/bindings";

// STUB(plan 05): Plan 05's `cost::CostLine`, `LiveCost` and `CostSummary` (A6, A19).
export type CostLine = {
  key: string;
  cost_usd: number;
  input_tokens: number;
  output_tokens: number;
  estimated: boolean;
};
export type LiveCost = { cost_usd: number; context_used_pct: number; at: string };
export type CostSummary = {
  cost_usd: number;
  input_tokens: number;
  output_tokens: number;
  estimated: boolean;
  live: LiveCost | null;
  by_project: CostLine[];
  by_task: CostLine[];
  by_stage: CostLine[];
  by_day: CostLine[];
  by_model: CostLine[];
  by_agent: CostLine[];
};

// STUB(plan 05): Plan 05's `changes::ChangedFile` and `RepoChanges`, the fields this plan reads.
export type ChangedFile = { path: string; added: number; removed: number };
export type RepoChanges = { repo_root: string; files: ChangedFile[] };

// STUB(plan 10): Plan 10's `jira::Issue`, the fields this plan reads.
export type Issue = { key: string; summary: string; status: string; url: string };

type Pending = {
  costSummary?(scope: Scope): Promise<Result<CostSummary, string>>;
  gitChanges?(taskId: TaskId): Promise<Result<RepoChanges[], string>>;
  sessionsTerminal?(projectId: ProjectId): Promise<Result<SessionId, string>>;
  jiraSearch?(projectId: ProjectId, query: string): Promise<Result<Issue[], string>>;
  workspacesCreate?(name: string, projectIds: ProjectId[]): Promise<Result<Workspace, string>>;
  workspacesAddProject?(id: WorkspaceId, path: string): Promise<Result<Workspace, string>>;
  workspacesSaveAs?(id: WorkspaceId, path: string): Promise<Result<Workspace, string>>;
};

const pending = () => commands as unknown as Pending;
const missing = <T>(name: string): Result<T, string> => ({ status: "error", error: `${name} is not available yet` });

/** STUB(plan 05): `cost_summary(scope) -> CostSummary`. */
export function costSummary(scope: Scope): Promise<Result<CostSummary, string>> {
  return pending().costSummary?.(scope) ?? Promise.resolve(missing("cost_summary"));
}

/** STUB(plan 05): `git_changes(task_id) -> Vec<RepoChanges>`. */
export function gitChanges(taskId: TaskId): Promise<Result<RepoChanges[], string>> {
  return pending().gitChanges?.(taskId) ?? Promise.resolve({ status: "ok", data: [] });
}

/** STUB(plan 04): `sessions_terminal(project_id) -> SessionId`. */
export function sessionsTerminal(projectId: ProjectId): Promise<Result<SessionId, string>> {
  return pending().sessionsTerminal?.(projectId) ?? Promise.resolve(missing("sessions_terminal"));
}

/** STUB(plan 10): `jira_search(project_id, query) -> Vec<Issue>`. */
export function jiraSearch(projectId: ProjectId, query: string): Promise<Result<Issue[], string>> {
  return pending().jiraSearch?.(projectId, query) ?? Promise.resolve({ status: "ok", data: [] });
}

/** STUB(plan 10): `workspaces_create(name, project_ids) -> Workspace`. */
export function workspacesCreate(name: string, projectIds: ProjectId[]): Promise<Result<Workspace, string>> {
  return pending().workspacesCreate?.(name, projectIds) ?? Promise.resolve(missing("workspaces_create"));
}

/** STUB(plan 10): `workspaces_add_project(id, path) -> Workspace`. */
export function workspacesAddProject(id: WorkspaceId, path: string): Promise<Result<Workspace, string>> {
  return pending().workspacesAddProject?.(id, path) ?? Promise.resolve(missing("workspaces_add_project"));
}

/** STUB(plan 10): `workspaces_save_as(id, path) -> Workspace`. */
export function workspacesSaveAs(id: WorkspaceId, path: string): Promise<Result<Workspace, string>> {
  return pending().workspacesSaveAs?.(id, path) ?? Promise.resolve(missing("workspaces_save_as"));
}
