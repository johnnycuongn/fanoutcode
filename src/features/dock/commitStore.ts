// The Commit tab's state per task: the drafted commits, Copy (which records CopyCommit) and Commit (only offered
// with "Fanout Code may commit" on, §6.6), and Draft again after a failed draft.
import type { TaskId } from "@/lib/bindings";
import { errorMessage, live } from "@/lib/ipcResult";
import { createStore } from "@/lib/stores/createStore";
import { copyText } from "@/livespec/clipboard";
import { draftsFrom, type CommitDraft } from "./commits";

export type CommitIpc = ReturnType<typeof live<"gitDrafts" | "tasksAction" | "tasksDraftCommits">>;
export interface CommitTaskState {
  drafts: CommitDraft[];
  error: string | null;
}

const systemClipboard = {
  async write(text: string): Promise<void> {
    if (!(await copyText(text))) throw "Couldn't copy. Select the text and copy it instead.";
  },
};

export function createCommitStore(ipc: CommitIpc, clipboard: { write(text: string): Promise<void> } = systemClipboard) {
  const store = createStore<Record<string, CommitTaskState>>({});
  const patch = (id: TaskId, p: Partial<CommitTaskState>) =>
    store.set((s) => ({ ...s, [id]: { ...(s[id] ?? { drafts: [], error: null }), ...p } }));
  return {
    store,
    async load(taskId: TaskId): Promise<void> {
      try {
        patch(taskId, { drafts: draftsFrom(await ipc.gitDrafts(taskId)), error: null });
      } catch (e) {
        patch(taskId, { error: errorMessage(e) });
      }
    },
    async copy(taskId: TaskId, draft: CommitDraft): Promise<boolean> {
      try {
        await clipboard.write(draft.message);
        await ipc.tasksAction(taskId, { type: "CopyCommit" });
        return true;
      } catch (e) {
        patch(taskId, { error: errorMessage(e) });
        return false;
      }
    },
    async redraft(taskId: TaskId): Promise<void> {
      try {
        patch(taskId, { error: null });
        await ipc.tasksDraftCommits(taskId);
      } catch (e) {
        patch(taskId, { error: errorMessage(e) });
      }
    },
    async commit(taskId: TaskId): Promise<void> {
      try {
        await ipc.tasksAction(taskId, { type: "Commit" });
      } catch (e) {
        patch(taskId, { error: errorMessage(e) });
      }
    },
  };
}

export type CommitStore = ReturnType<typeof createCommitStore>;

export const commitStore = createCommitStore(live("gitDrafts", "tasksAction", "tasksDraftCommits"));
