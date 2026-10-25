// Jira for the webview: link status per project, issues keyed `<projectId>:<KEY>`, the per-project cache, a
// debounced search, comments, transitions and creating a task from an issue. The API token is passed straight to
// the link command and never kept here. Plan 08's Ticket tab and Plan 06's Ticket section read this store.
import {
  commands,
  type CachedIssue,
  type Issue,
  type JiraLinkStatus,
  type ProjectId,
  type Task,
  type Transition,
  type WorkspaceId,
} from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { messageOf, unwrap } from "./result";

export type JiraState = {
  status: Record<ProjectId, JiraLinkStatus>;
  issues: Record<string, Issue>;
  cached: Record<ProjectId, CachedIssue[]>;
  search: { query: string; results: Issue[]; pending: boolean };
  error: string | null;
};

export type JiraScope = { project: ProjectId } | { workspace: WorkspaceId };

const base = createStore<JiraState>({
  status: {},
  issues: {},
  cached: {},
  search: { query: "", results: [], pending: false },
  error: null,
});

const SEARCH_DEBOUNCE_MS = 300;
let searchTimer: ReturnType<typeof setTimeout> | null = null;

export const issueKey = (projectId: ProjectId, key: string) => `${projectId}:${key}`;

/** Runs a command; an error is kept for the screen and rethrown to the caller. */
async function call<T>(p: Promise<Parameters<typeof unwrap<T>>[0]>): Promise<T> {
  try {
    const data = unwrap(await p);
    base.set({ error: null });
    return data;
  } catch (e) {
    base.set({ error: messageOf(e) });
    throw e;
  }
}

function putIssue(projectId: ProjectId, issue: Issue) {
  base.set((s) => ({ ...s, issues: { ...s.issues, [issueKey(projectId, issue.key)]: issue } }));
}

export const jiraStore = {
  ...base,
  async link(scope: JiraScope, siteUrl: string, email: string, token: string): Promise<void> {
    if ("project" in scope) {
      await call(commands.jiraLink(scope.project, siteUrl, email, token));
      await jiraStore.loadStatus(scope.project);
    } else {
      await call(commands.jiraLinkWorkspace(scope.workspace, siteUrl, email, token));
    }
  },
  async unlink(scope: JiraScope): Promise<void> {
    const s =
      "project" in scope
        ? { workspace: null, project: scope.project, task: null }
        : { workspace: scope.workspace, project: null, task: null };
    await call(commands.jiraUnlink(s));
    if ("project" in scope) await jiraStore.loadStatus(scope.project);
  },
  async loadStatus(projectId: ProjectId): Promise<JiraLinkStatus> {
    const status = await call(commands.jiraStatus(projectId));
    base.set((s) => ({ ...s, status: { ...s.status, [projectId]: status } }));
    return status;
  },
  async loadCached(projectId: ProjectId): Promise<CachedIssue[]> {
    const cached = await call(commands.jiraCached(projectId));
    base.set((s) => ({ ...s, cached: { ...s.cached, [projectId]: cached } }));
    for (const c of cached) if (!base.get().issues[issueKey(projectId, c.issue.key)]) putIssue(projectId, c.issue);
    return cached;
  },
  /** The issue now: the cached one at once (refreshed in the background), else a live fetch. */
  async issue(projectId: ProjectId, key: string): Promise<Issue> {
    const fresh = call(commands.jiraIssue(projectId, key)).then((i) => {
      putIssue(projectId, i);
      return i;
    });
    const known = base.get().issues[issueKey(projectId, key)];
    if (known) {
      fresh.catch(() => undefined);
      return known;
    }
    return fresh;
  },
  search(projectId: ProjectId, query: string): Promise<void> {
    base.set((s) => ({ ...s, search: { ...s.search, query, pending: true } }));
    if (searchTimer) clearTimeout(searchTimer);
    return new Promise((resolve) => {
      searchTimer = setTimeout(() => {
        searchTimer = null;
        void call(commands.jiraSearch(projectId, query))
          .then((results) => {
            if (base.get().search.query === query)
              base.set((s) => ({ ...s, search: { query, results, pending: false } }));
          })
          .catch(() => base.set((s) => ({ ...s, search: { ...s.search, pending: false } })))
          .finally(resolve);
      }, SEARCH_DEBOUNCE_MS);
    });
  },
  comment: (projectId: ProjectId, key: string, body: string): Promise<null> =>
    call(commands.jiraComment(projectId, key, body)),
  createTask: (projectId: ProjectId, key: string, workspaceId?: WorkspaceId | null): Promise<Task> =>
    call(commands.jiraCreateTask(projectId, key, workspaceId ?? null)),
  transitions: (projectId: ProjectId, key: string): Promise<Transition[]> =>
    call(commands.jiraTransitions(projectId, key)),
  transition: (projectId: ProjectId, key: string, id: string): Promise<null> =>
    call(commands.jiraTransition(projectId, key, id)),
  reset() {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = null;
    base.reset();
  },
};
