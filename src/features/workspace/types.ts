// Types of the workspace feature; IPC shapes come from the generated bindings.
import type { Workspace, WorkspaceStatus } from "@/lib/bindings";

export type WorkspaceState = {
  workspace: Workspace | null;
  status: WorkspaceStatus | null;
  fragments: Record<string, string>;
  agents: Record<string, string>;
  /** The file changed on disk (explicit-save mode); the screen offers Reload / Keep mine. */
  externalChange: boolean;
  busy: boolean;
  error: string | null;
};

/** The stages that take a workspace prompt fragment, as the shell names them. */
export const FRAGMENT_STAGES = ["spec", "plan", "build", "review", "test", "commit"] as const;
export type FragmentStage = (typeof FRAGMENT_STAGES)[number];
