// Opening a workspace file that may come from someone else: preview first (nothing is written), then the import
// dialog when the file brings anything new, and only then the window. A file this app wrote for the record at that
// path, unchanged since, opens straight away.
import { toast } from "@/components/ui";
import { commands, type WorkspaceImportPreview } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { messageOf, unwrap } from "./result";

export type ImportState = {
  path: string | null;
  preview: WorkspaceImportPreview | null;
  /** File and folder references the developer ticked (by preview id). */
  approved: string[];
  busy: boolean;
  error: string | null;
};

const base = createStore<ImportState>({ path: null, preview: null, approved: [], busy: false, error: null });

export const importStore = {
  ...base,
  toggle(docId: string) {
    const approved = base.get().approved;
    base.set({ approved: approved.includes(docId) ? approved.filter((d) => d !== docId) : [...approved, docId] });
  },
  cancel() {
    base.reset();
  },
  async confirm(): Promise<boolean> {
    const { path, preview, approved } = base.get();
    if (!path || !preview || preview.blocked) return false;
    base.set({ busy: true, error: null });
    try {
      unwrap(await commands.workspacesConfirmOpen(preview.token, approved));
      unwrap(await commands.windowsOpenWorkspace(path));
      base.reset();
      return true;
    } catch (e) {
      base.set({ busy: false, error: messageOf(e) });
      return false;
    }
  },
};

/** Opens the workspace file at `path`, through the import dialog when it needs review. */
export async function openWorkspaceFile(path: string): Promise<void> {
  try {
    const preview = unwrap(await commands.workspacesPreviewOpen(path));
    if (!preview.needs_confirmation) {
      unwrap(await commands.windowsOpenWorkspace(path));
      return;
    }
    base.set({ path, preview, approved: [], busy: false, error: null });
  } catch (e) {
    toast.error(messageOf(e));
  }
}
