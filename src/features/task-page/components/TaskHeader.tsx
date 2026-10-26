// The task page header (§6, §10.3): back, project badge in workspaces, the title with inline rename, the stepper and
// the hand-back chip, the cost meter, the primary action and the overflow menu, and the "links changed" hint (§4.1).
import { ArrowLeft } from "lucide-react";
import { useState, type ReactElement } from "react";
import { Badge, Button, Chip, IconButton, Input } from "@/components/ui";
import type { CostSummary, Project, StageAction, Task } from "@/lib/bindings";
import { asSwatch, handbackLabel } from "@/lib/format";
import { useUncommitted } from "@/lib/uncommitted";
import { handbackKind } from "../agentState";
import type { PrimaryButton } from "../primaryAction";
import type { TaskSettings } from "../types";
import { CostMeter } from "./CostMeter";
import { OverflowMenu } from "./OverflowMenu";
import { PrimaryAction } from "./PrimaryAction";
import { StageStepper } from "./StageStepper";

export const LINKS_CHANGED = "Links changed, start a fresh session to apply";

export interface TaskHeaderProps {
  task: Task;
  project: Project | null;
  showProjectBadge: boolean;
  settings: TaskSettings;
  cost: CostSummary | undefined;
  contextPct: number | undefined;
  primary: PrimaryButton;
  linksChanged: boolean;
  renaming: boolean;
  onRenamingChange(on: boolean): void;
  onPrimary(): void;
  onBack(): void;
  onRename(title: string): void;
  onAction(a: StageAction): void;
  onStopAgent(): void;
  onLinkIssue(): void;
  onToggleWorktree?(): void;
  onIntegrate?(): void;
}

function RenameInput({ title, onDone, onCancel }: { title: string; onDone(t: string): void; onCancel(): void }) {
  const [value, setValue] = useState(title);
  return (
    <Input
      autoFocus
      size="sm"
      aria-label="Task title"
      className="w-72"
      value={value}
      onChange={(e) => setValue(e.target.value)}
      onBlur={onCancel}
      onKeyDown={(e) => {
        if (e.key === "Enter" && value.trim()) onDone(value.trim());
        if (e.key === "Escape") onCancel();
      }}
    />
  );
}

function HandbackChip({ task }: { task: Task }) {
  const kind = handbackKind(task.state);
  if (kind === null) return null;
  return (
    <Chip size="sm" data-testid="handback-chip" tone={kind === "TestFailed" || kind === "Error" ? "bad" : "warn"}>
      {handbackLabel(task)}
    </Chip>
  );
}

/** Done, but some of the task's files are still only in the working tree. */
function UncommittedChip({ task }: { task: Task }) {
  const n = useUncommitted(task.id, task.stage === "Done");
  if (n === 0) return null;
  return (
    <Chip size="sm" tone="warn">
      {n === 1 ? "1 file not committed" : `${n} files not committed`}
    </Chip>
  );
}

export function TaskHeader(p: TaskHeaderProps): ReactElement {
  const testsRun = p.settings.testEnabled && Boolean(p.settings.testCommand?.trim());
  const { task } = p;
  return (
    <header className="shrink-0 border-b border-line bg-bar px-3 py-1.5">
      <div className="flex min-w-0 items-center gap-2">
        <IconButton
          size="sm"
          label="Back to board"
          icon={<ArrowLeft aria-hidden className="size-4" />}
          onClick={p.onBack}
        />
        {p.showProjectBadge && p.project && (
          <Badge size="sm" swatch={asSwatch(p.project.color)}>
            {p.project.name}
          </Badge>
        )}
        {p.renaming ? (
          <RenameInput
            title={task.title}
            onDone={(t) => {
              p.onRename(t);
              p.onRenamingChange(false);
            }}
            onCancel={() => p.onRenamingChange(false)}
          />
        ) : (
          <Button
            variant="ghost"
            size="sm"
            aria-label={`Rename ${task.title}`}
            className="min-w-0 truncate font-semibold text-ink"
            onClick={() => p.onRenamingChange(true)}
          >
            {task.title}
          </Button>
        )}
        <StageStepper stage={task.stage} skipTest={!testsRun} />
        <HandbackChip task={task} />
        <UncommittedChip task={task} />
        <span className="flex-1" />
        {p.settings.costOverview && <CostMeter cost={p.cost} contextPct={p.contextPct} />}
        <PrimaryAction
          primary={p.primary}
          stage={task.stage}
          mayCommit={p.settings.mayCommit}
          onPrimary={p.onPrimary}
          onAction={p.onAction}
        />
        <OverflowMenu
          task={task}
          mayCommit={p.settings.mayCommit}
          onRename={() => p.onRenamingChange(true)}
          onAction={p.onAction}
          onStopAgent={p.onStopAgent}
          onLinkIssue={p.onLinkIssue}
          {...(p.onToggleWorktree ? { onToggleWorktree: p.onToggleWorktree } : {})}
          {...(p.onIntegrate ? { onIntegrate: p.onIntegrate } : {})}
        />
      </div>
      {p.linksChanged && <p className="mt-1 text-xs text-warn">{LINKS_CHANGED}</p>}
    </header>
  );
}
