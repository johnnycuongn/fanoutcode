// The linking copy shared by the settings screen, the task page and the new-task sheet. The policy labels are the
// same strings as `fanoutcode_core::workspace::policy`.
import type { AgentKind, LinkStatus } from "@/lib/bindings";

export const READ_ONLY_LABEL = "Read only: file tools blocked, shell commands checked";
export const READ_AND_MODIFY_LABEL = "Read and modify on request";
export const POLICY_APPLIES_NOTE = "Applies to sessions started from now on.";
export const LINKS_CHANGED_MESSAGE = "Links changed, start a fresh session to apply";

/** Codex and shell tasks see only their home project. */
export function linkingNote(kind: AgentKind, inWorkspace: boolean): string | null {
  if (!inWorkspace || kind === "ClaudeCode") return null;
  return "Linked projects are available to Claude Code tasks only. A Codex or shell task sees only its home project.";
}

export function linkStatusMessage(status: LinkStatus): string | null {
  return status.type === "Changed" ? LINKS_CHANGED_MESSAGE : null;
}
