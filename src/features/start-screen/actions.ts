// What the start screen does: open a project or workspace (from a picker or a recent entry), and turn the open
// project windows into one new workspace.
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { toast } from "@/components/ui";
import { commands, type RecentEntry, type Result, type Workspace, type WorkspaceId } from "@/lib/bindings";
import type { ProjectId, WindowId, WindowInfo } from "@/lib/bindings";
import { workspacesCreate, workspacesSaveAs } from "@/lib/pendingCommands";

function report<T>(r: Result<T, string> | undefined): T | null {
  if (r?.status === "ok") return r.data;
  if (r) toast.error(r.error);
  return null;
}

export async function openProjectDialog(): Promise<void> {
  const path = await openDialog({ directory: true, title: "Open a project folder" });
  if (typeof path === "string") report(await commands.windowsOpenProject(path));
}

export async function openWorkspaceDialog(): Promise<void> {
  const path = await openDialog({
    title: "Open a workspace",
    filters: [{ name: "Team Code workspace", extensions: ["json"] }],
  });
  if (typeof path === "string") report(await commands.windowsOpenWorkspace(path));
}

export async function openRecent(entry: RecentEntry): Promise<void> {
  const r =
    entry.kind === "Project"
      ? await commands.windowsOpenProject(entry.path)
      : await commands.windowsOpenWorkspace(entry.path);
  report(r);
}

/** Missing entry: pick where it went and open it from there. */
export async function relocateRecent(entry: RecentEntry): Promise<void> {
  if (entry.kind === "Project") return openProjectDialog();
  return openWorkspaceDialog();
}

/** Missing workspace file: write the record out again (Plan 10's `workspaces_save_as`). */
export async function reexportWorkspace(entry: RecentEntry): Promise<void> {
  const path = await saveDialog({ defaultPath: `${entry.name}.teamcode-workspace.json` });
  if (typeof path === "string") report(await workspacesSaveAs(entry.id, path));
}

export type NewWorkspaceDeps = {
  windowsList(): Promise<WindowInfo[]>;
  workspacesCreate(name: string, projectIds: ProjectId[]): Promise<Pick<Workspace, "id" | "name" | "file_path">>;
  windowsOpenWorkspace(path: string): Promise<unknown>;
  windowsClose(windowId: WindowId): Promise<unknown>;
};

const unwrap = async <T>(p: Promise<Result<T, string>>): Promise<T> => {
  const r = await p;
  if (r.status === "error") throw new Error(r.error);
  return r.data;
};

export const realDeps: NewWorkspaceDeps = {
  windowsList: () => commands.windowsList(),
  workspacesCreate: (name, ids) => unwrap(workspacesCreate(name, ids)),
  windowsOpenWorkspace: (path) => unwrap(commands.windowsOpenWorkspace(path)),
  windowsClose: (id) => unwrap(commands.windowsClose(id)),
};

/** A workspace of every open project window's project (distinct, in window order), opened in place of them. */
export async function newWorkspaceFromOpenWindows(deps: NewWorkspaceDeps = realDeps): Promise<WorkspaceId | null> {
  const projectWindows = (await deps.windowsList()).filter((w) => w.container?.type === "Project");
  const ids = [...new Set(projectWindows.flatMap((w) => w.project_ids))];
  if (ids.length < 1) return null;
  const workspace = await deps.workspacesCreate(`${projectWindows[0].title} workspace`, ids);
  if (workspace.file_path) await deps.windowsOpenWorkspace(workspace.file_path);
  for (const w of projectWindows) await deps.windowsClose(w.window_id);
  return workspace.id;
}
