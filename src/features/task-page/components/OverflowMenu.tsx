// The header's "…" menu: rename, the forward step, fresh session, stop, back a stage, worktree, Jira, archive.
import { MoreHorizontal } from "lucide-react";
import type { ReactElement } from "react";
import { IconButton, Menu, MenuContent, MenuItem, MenuSeparator, MenuTrigger } from "@/components/ui";
import type { StageAction, Task } from "@/lib/bindings";
import { forwardActionFor, freshSessionAllowed, PREVIOUS_STAGE } from "../primaryAction";

export interface OverflowMenuProps {
  task: Task;
  mayCommit: boolean;
  onRename(): void;
  onAction(a: StageAction): void;
  onStopAgent(): void;
  onLinkIssue(): void;
  onToggleWorktree?(): void;
}

export function OverflowMenu({
  task,
  mayCommit,
  onRename,
  onAction,
  onStopAgent,
  onLinkIssue,
  onToggleWorktree,
}: OverflowMenuProps): ReactElement {
  const forward = task.archived ? null : forwardActionFor(task.stage, { mayCommit });
  const previous = PREVIOUS_STAGE[task.stage];
  const started = task.primary_session !== null;
  return (
    <Menu>
      <MenuTrigger asChild>
        <IconButton size="sm" label="More actions" icon={<MoreHorizontal aria-hidden className="size-4" />} />
      </MenuTrigger>
      <MenuContent align="end" minWidth={220}>
        <MenuItem shortcut="cmd+e" onSelect={onRename}>
          Rename…
        </MenuItem>
        {forward && <MenuItem onSelect={() => onAction(forward.action)}>{forward.label}</MenuItem>}
        {freshSessionAllowed(task.stage) && (
          <MenuItem onSelect={() => onAction({ type: "StartFresh" })}>Start fresh session</MenuItem>
        )}
        {started && <MenuItem onSelect={onStopAgent}>Stop agent</MenuItem>}
        {previous && !task.archived && (
          <MenuItem onSelect={() => onAction({ type: "Back", stage: previous })}>Back to {previous}</MenuItem>
        )}
        <MenuItem
          disabled={started || !onToggleWorktree}
          hint={started ? "Changes with Start fresh session" : undefined}
          onSelect={() => onToggleWorktree?.()}
        >
          {task.worktree === "On" ? "Turn worktree off" : "Use a worktree"}
        </MenuItem>
        <MenuItem onSelect={onLinkIssue}>Link Jira issue…</MenuItem>
        <MenuSeparator />
        {task.archived ? (
          <MenuItem onSelect={() => onAction({ type: "Restore" })}>Restore</MenuItem>
        ) : (
          <MenuItem danger shortcut="cmd+shift+a" onSelect={() => onAction({ type: "Archive" })}>
            Archive
          </MenuItem>
        )}
      </MenuContent>
    </Menu>
  );
}
