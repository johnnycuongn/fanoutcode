// First run, step 2: where each task's agent works. Two selectable cards, the recommended one first.
import type { ReactElement } from "react";
import { Button, cx } from "@/components/ui";
import type { WorktreeConvention } from "@/lib/bindings";

const OPTIONS: Array<{ value: WorktreeConvention; title: string; body: string }> = [
  {
    value: "FanoutCodeManaged",
    title: "Its own worktree (recommended)",
    body: "Each task works in its own copy of the repo under .worktrees/, on its own branch. Tasks running at the same time never touch each other's files, and nothing is deleted without asking.",
  },
  {
    value: "ClaudeCode",
    title: "Claude Code's worktrees",
    body: "Claude Code makes a worktree for each session under .claude/worktrees/ and offers to remove it when the session ends.",
  },
];

export function ConventionChoice({
  value,
  onChange,
}: {
  value: WorktreeConvention;
  onChange(v: WorktreeConvention): void;
}): ReactElement {
  return (
    <div role="group" aria-label="Worktree lifecycle" className="grid gap-3 sm:grid-cols-2">
      {OPTIONS.map((o) => (
        <Button
          key={o.value}
          variant="secondary"
          aria-pressed={value === o.value}
          onClick={() => onChange(o.value)}
          className={cx(
            "h-auto flex-col items-start gap-1 whitespace-normal p-3 text-left",
            value === o.value && "border-accent bg-accent/10",
          )}
        >
          <span className="font-medium text-ink">{o.title}</span>
          <span className="text-xs font-normal text-mute">{o.body}</span>
        </Button>
      ))}
    </div>
  );
}
