// The Ticket tab's state per task: the linked Jira issue, search results to link one, and comment posting. Errors
// (e.g. no Jira site linked) are kept verbatim for the tab to show.
import type { Issue, Task, TaskId, TaskPatch } from "@/lib/bindings";
import { errorMessage, live, type Live } from "@/lib/ipcResult";
import { createStore } from "@/lib/stores/createStore";

export type TicketIpc = Live<"jiraIssue" | "jiraComment" | "jiraSearch" | "tasksUpdate">;

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
export interface TicketTaskState {
  issue: Issue | null;
  results: Issue[];
  loading: boolean;
  posting: boolean;
  error: string | null;
}

const fresh = (): TicketTaskState => ({ issue: null, results: [], loading: false, posting: false, error: null });

export function createTicketStore(ipc: TicketIpc) {
  const store = createStore<Record<string, TicketTaskState>>({});
  const patch = (id: TaskId, p: Partial<TicketTaskState>) =>
    store.set((s) => ({ ...s, [id]: { ...(s[id] ?? fresh()), ...p } }));

  async function loadKey(task: Task, key: string): Promise<void> {
    patch(task.id, { loading: true, error: null });
    try {
      patch(task.id, { issue: await ipc.jiraIssue(task.project_id, key), loading: false });
    } catch (e) {
      patch(task.id, { loading: false, error: errorMessage(e) });
    }
  }

  return {
    store,
    async load(task: Task): Promise<void> {
      if (!task.jira_key) {
        patch(task.id, { issue: null });
        return;
      }
      await loadKey(task, task.jira_key);
    },
    async comment(task: Task, body: string): Promise<boolean> {
      if (!task.jira_key) return false;
      patch(task.id, { posting: true, error: null });
      try {
        await ipc.jiraComment(task.project_id, task.jira_key, body);
        patch(task.id, { posting: false });
        return true;
      } catch (e) {
        patch(task.id, { posting: false, error: errorMessage(e) });
        return false;
      }
    },
    async search(task: Task, query: string): Promise<void> {
      patch(task.id, { error: null });
      try {
        patch(task.id, { results: await ipc.jiraSearch(task.project_id, query) });
      } catch (e) {
        patch(task.id, { error: errorMessage(e) });
      }
    },
    async link(task: Task, key: string): Promise<void> {
      try {
        await ipc.tasksUpdate(task.id, jiraKeyPatch(key));
      } catch (e) {
        patch(task.id, { error: errorMessage(e) });
        return;
      }
      await loadKey(task, key);
    },
  };
}

export type TicketStore = ReturnType<typeof createTicketStore>;

export const ticketStore = createTicketStore(live("jiraIssue", "jiraComment", "jiraSearch", "tasksUpdate"));
