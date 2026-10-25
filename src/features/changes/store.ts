// The Changes tab's state per task (§6.4): the change set by repo, the review notes, the selected file with its diff
// and the editable working-tree buffer, and the save round trip. A `git-changes` event while the buffer is dirty
// marks it stale on disk instead of replacing it (Review Focus 2); the editor then offers Reload or Overwrite.
import type { FileDiff, RepoChanges, TaskId } from "@/lib/bindings";
import { errorMessage, live, type Live } from "@/lib/ipcResult";
import { createStore } from "@/lib/stores/createStore";
import { parseReviewNotes } from "./reviewNotes";
import type { ReviewNote } from "./types";

export type DiffMode = "unified" | "split";
/** `repo` is `RepoChanges.repo_root`. */
export interface Selection {
  repo: string;
  path: string;
}
export interface ChangesTaskState {
  repos: RepoChanges[];
  notes: ReviewNote[];
  selected: Selection | null;
  diff: FileDiff | null;
  buffer: string;
  dirty: boolean;
  staleOnDisk: boolean;
  saving: boolean;
  loading: boolean;
  error: string | null;
}
export interface ChangesState {
  byTask: Record<string, ChangesTaskState>;
  mode: DiffMode;
}
export type ChangesIpc = Live<
  "gitChanges" | "gitDiff" | "gitFileWrite" | "gitIncludeOther" | "memoryRead" | "tasksAction"
>;
export interface ModePersist {
  read(): DiffMode | null;
  write(m: DiffMode): void;
}

const MODE_KEY = "fanoutcode.changes.mode";

export const localModePersist: ModePersist = {
  read() {
    try {
      const v = globalThis.localStorage?.getItem(MODE_KEY);
      return v === "split" || v === "unified" ? v : null;
    } catch {
      return null;
    }
  },
  write(m) {
    try {
      globalThis.localStorage?.setItem(MODE_KEY, m);
    } catch {
      // blocked storage: the choice lasts for this session
    }
  },
};

/** The last path segment of a repo root. */
export function repoName(repoRoot: string): string {
  const parts = repoRoot.replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || repoRoot;
}

export function replyPrompt(note: ReviewNote, reply: string): string {
  return `Review note ${note.location}:${note.line} · ${note.severity} · ${note.text}\n\nReply from the developer: ${reply}`;
}

const fresh = (): ChangesTaskState => ({
  repos: [],
  notes: [],
  selected: null,
  diff: null,
  buffer: "",
  dirty: false,
  staleOnDisk: false,
  saving: false,
  loading: false,
  error: null,
});

export function createChangesStore(ipc: ChangesIpc, persist: ModePersist = localModePersist) {
  const store = createStore<ChangesState>({ byTask: {}, mode: persist.read() ?? "unified" });
  const get = (id: TaskId) => store.get().byTask[id] ?? fresh();
  const patch = (id: TaskId, p: Partial<ChangesTaskState>) =>
    store.set((s) => ({ ...s, byTask: { ...s.byTask, [id]: { ...(s.byTask[id] ?? fresh()), ...p } } }));

  async function refresh(taskId: TaskId): Promise<void> {
    try {
      patch(taskId, { repos: await ipc.gitChanges(taskId) });
    } catch (e) {
      patch(taskId, { error: errorMessage(e) });
    }
  }

  async function loadDiff(taskId: TaskId, sel: Selection): Promise<void> {
    patch(taskId, { loading: true });
    try {
      const diff = await ipc.gitDiff(taskId, sel.repo, sel.path);
      const now = get(taskId).selected;
      if (now?.repo !== sel.repo || now.path !== sel.path) return; // another file was picked meanwhile
      patch(taskId, { diff, buffer: diff.after, dirty: false, staleOnDisk: false, loading: false, error: null });
    } catch (e) {
      patch(taskId, { loading: false, error: errorMessage(e) });
    }
  }

  async function onReviewChanged(taskId: TaskId): Promise<void> {
    try {
      const md = await ipc.memoryRead(taskId, "review.md");
      patch(taskId, { notes: typeof md === "string" ? parseReviewNotes(md) : [] });
    } catch {
      patch(taskId, { notes: [] });
    }
  }

  async function select(taskId: TaskId, sel: Selection): Promise<void> {
    patch(taskId, { selected: sel });
    await loadDiff(taskId, sel);
  }

  return {
    store,
    refresh,
    select,
    onReviewChanged,
    async open(taskId: TaskId): Promise<void> {
      await Promise.all([refresh(taskId), onReviewChanged(taskId)]);
      const st = get(taskId);
      if (st.selected) return;
      const first = st.repos.find((r) => r.files.length > 0);
      if (first) await select(taskId, { repo: first.repo_root, path: first.files[0].path });
    },
    edit(taskId: TaskId, text: string): void {
      patch(taskId, { buffer: text, dirty: true });
    },
    async save(taskId: TaskId): Promise<boolean> {
      const st = get(taskId);
      if (!st.selected) return false;
      const { repo, path } = st.selected;
      patch(taskId, { saving: true });
      try {
        await ipc.gitFileWrite(taskId, repo, path, st.buffer);
      } catch (e) {
        patch(taskId, { saving: false, error: errorMessage(e) });
        return false;
      }
      await loadDiff(taskId, st.selected);
      await refresh(taskId);
      patch(taskId, { saving: false });
      return true;
    },
    async reload(taskId: TaskId): Promise<void> {
      const sel = get(taskId).selected;
      if (sel) await loadDiff(taskId, sel);
    },
    async include(taskId: TaskId, sel: Selection): Promise<void> {
      try {
        await ipc.gitIncludeOther(taskId, sel.repo, sel.path);
      } catch (e) {
        patch(taskId, { error: errorMessage(e) });
        return;
      }
      await refresh(taskId);
    },
    setMode(mode: DiffMode): void {
      store.set({ mode });
      persist.write(mode);
    },
    async reply(taskId: TaskId, note: ReviewNote, text: string): Promise<void> {
      await ipc.tasksAction(taskId, { type: "Reply", prompt: replyPrompt(note, text) });
    },
    async onGitChanges(taskId: TaskId): Promise<void> {
      await refresh(taskId);
      const st = get(taskId);
      if (!st.selected) return;
      if (st.dirty) patch(taskId, { staleOnDisk: true });
      else await loadDiff(taskId, st.selected);
    },
  };
}

export type ChangesStore = ReturnType<typeof createChangesStore>;

export const changesStore = createChangesStore(
  live("gitChanges", "gitDiff", "gitFileWrite", "gitIncludeOther", "memoryRead", "tasksAction"),
);
