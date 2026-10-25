// Seeds the workspace store for component tests.
import type { Workspace, WorkspaceProject, WorkspaceStatus } from "@/lib/bindings";
import { workspaceStore } from "../store";
import type { WorkspaceState } from "../types";

export function wp(id: string, over: Partial<WorkspaceProject> = {}): WorkspaceProject {
  return { project_id: id, order: 0, protected: false, ...over };
}

export function mockWorkspace(
  over: Partial<Workspace> = {},
  status: WorkspaceStatus = { file_missing: false, missing_projects: [], explicit_save: false },
  state: Partial<WorkspaceState> = {},
): void {
  workspaceStore.set({
    workspace: {
      id: "ws-1",
      name: "Shop",
      file_path: "/app/workspaces/ws-1/Shop.teamcode-workspace.json",
      dirty: false,
      projects: [],
      policy: "ReadAndModifyOnRequest",
      ...over,
    },
    status,
    fragments: {},
    agents: {},
    externalChange: false,
    busy: false,
    error: null,
    ...state,
  });
}
