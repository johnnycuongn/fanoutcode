// The workspace window's record: membership, policy, fragments, agents and the file's save state. Every action calls
// one `workspaces_*` command and replaces the record with the one returned; `workspace-changed` keeps other windows
// in step and `workspace-file-changed` raises the reload banner.
import { commands, events, type LinkPolicy, type ProjectId, type Workspace, type WorkspaceId } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { messageOf, unwrap } from "./result";
import type { WorkspaceState } from "./types";

const initial: WorkspaceState = {
  workspace: null,
  status: null,
  fragments: {},
  agents: {},
  externalChange: false,
  busy: false,
  error: null,
};

const base = createStore<WorkspaceState>(initial);
let listening = false;

function listen() {
  if (listening) return;
  listening = true;
  void events.workspaceChanged.listen((e) => {
    const current = base.get().workspace;
    if (current && e.payload.workspace.id === current.id) {
      base.set({ workspace: e.payload.workspace });
      void refreshStatus();
    }
  });
  void events.workspaceFileChanged.listen((e) => {
    if (base.get().workspace?.id === e.payload.workspace_id) base.set({ externalChange: true });
  });
}

const currentId = (): WorkspaceId | null => base.get().workspace?.id ?? null;

async function refreshStatus() {
  const id = currentId();
  if (!id) return;
  try {
    base.set({ status: unwrap(await commands.workspacesStatus(id)) });
  } catch {
    // the status is advisory; the next change retries
  }
}

/** Runs one command against the loaded workspace and takes the record it returns. */
async function run(
  call: (id: WorkspaceId) => Promise<Parameters<typeof unwrap<Workspace>>[0]>,
): Promise<Workspace | null> {
  const id = currentId();
  if (!id) return null;
  base.set({ busy: true, error: null });
  try {
    const ws = unwrap(await call(id));
    base.set({ workspace: ws, busy: false });
    void refreshStatus();
    return ws;
  } catch (e) {
    base.set({ busy: false, error: messageOf(e) });
    return null;
  }
}

export const workspaceStore = {
  ...base,
  reset() {
    base.reset();
  },
  async load(id: WorkspaceId): Promise<void> {
    listen();
    base.set({ busy: true, error: null });
    try {
      const all = unwrap(await commands.workspacesList());
      const workspace = all.find((w) => w.id === id) ?? null;
      if (!workspace) throw new Error("This workspace is no longer known.");
      const [status, fragments, agents] = await Promise.all([
        commands.workspacesStatus(id),
        commands.workspacesFragments(id),
        commands.workspacesAgents(id),
      ]);
      base.set({
        workspace,
        status: unwrap(status),
        fragments: unwrap(fragments) as Record<string, string>,
        agents: unwrap(agents) as Record<string, string>,
        busy: false,
      });
    } catch (e) {
      base.set({ busy: false, error: messageOf(e) });
    }
  },
  addProject: (path: string) => run((id) => commands.workspacesAddProject(id, path)),
  removeProject: (projectId: ProjectId) => run((id) => commands.workspacesRemoveProject(id, projectId)),
  reorder: (projectIds: ProjectId[]) => run((id) => commands.workspacesReorderProjects(id, projectIds)),
  setPolicy: (policy: LinkPolicy, protectedIds: ProjectId[]) =>
    run((id) => commands.workspacesSetPolicy(id, policy, protectedIds)),
  async setFragment(stage: string, text: string | null) {
    const ws = await run((id) => commands.workspacesSetFragment(id, stage, text));
    if (ws) {
      const fragments = { ...base.get().fragments };
      if (text && text.trim()) fragments[stage] = text.trim();
      else delete fragments[stage];
      base.set({ fragments });
    }
    return ws;
  },
  async setAgent(name: string, markdown: string | null) {
    const ws = await run((id) => commands.workspacesSetAgent(id, name, markdown));
    if (ws) {
      const agents = { ...base.get().agents };
      if (markdown && markdown.trim()) agents[name] = markdown.trim();
      else delete agents[name];
      base.set({ agents });
    }
    return ws;
  },
  save: () => run((id) => commands.workspacesSave(id)),
  saveAs: (path: string) => run((id) => commands.workspacesSaveAs(id, path)),
  async reload(discardUnsaved: boolean) {
    const ws = await run((id) => commands.workspacesReload(id, discardUnsaved));
    if (ws) base.set({ externalChange: false });
    return ws;
  },
  /** Keep mine: dismiss the banner; the next save overwrites the file. */
  keepMine() {
    base.set({ externalChange: false });
  },
  relocateFile: (path: string) => run((id) => commands.workspacesRelocateFile(id, path)),
  relocateProject: (projectId: ProjectId, path: string) =>
    run((id) => commands.workspacesRelocateProject(id, projectId, path)),
  isDirty: () => base.get().workspace?.dirty ?? false,
  isExplicitSave: () => base.get().status?.explicit_save ?? false,
  missingProjects: (): ProjectId[] => base.get().status?.missing_projects ?? [],
};
