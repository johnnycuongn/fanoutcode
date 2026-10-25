// First run, step 1: which worktree lifecycle new tasks use. Two selectable cards.
import type { ReactElement } from "react";
import { Button, cx } from "@/components/ui";
import type { WorktreeConvention } from "@/lib/bindings";

const OPTIONS: Array<{ value: WorktreeConvention; title: string; body: string }> = [
  {
    value: "ClaudeCode",
    title: "Claude Code's own worktrees",
    body: "Launched with claude --worktree under .claude/worktrees/. A clean worktree is removed when its session exits; a dirty one gets Claude's keep-or-remove prompt in the terminal; stale worktrees are swept by Claude Code; claude rm removes a session's worktree when safe.",
  },
  {
    value: "FanoutCodeManaged",
    title: "Fanout Code-managed",
    body: "git worktree add <root>/.worktrees/<task-slug> -b fo/<task-slug>. Removing a worktree on archive is offered, never automatic.",
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
