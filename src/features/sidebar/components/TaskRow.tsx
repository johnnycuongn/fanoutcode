// A task in the sidebar: a 2 px edge in the task colour, its title and its state chip. Click opens it in the current
// view; ⌘-click in a new view.
import type { ReactElement } from "react";
import { Button, Chip, cx } from "@/components/ui";
import type { Task } from "@/lib/bindings";
import { handbackLabel } from "@/lib/format";
import { swatchClass } from "@/lib/theme";
import { openTaskInCurrentView, openTaskInNewView } from "@/features/shell";

function StateChip({ task }: { task: Task }): ReactElement {
  if (task.state.type === "NeedsYou")
    return (
      <Chip size="sm" tone="warn">
        {handbackLabel(task)}
      </Chip>
    );
  if (task.state.type === "Working")
    return (
      <Chip size="sm" status="working">
        working
      </Chip>
    );
  return (
    <Chip size="sm" tone="neutral">
      idle
    </Chip>
  );
}

export function TaskRow({ task }: { task: Task }): ReactElement {
  return (
    <Button
      variant="ghost"
      size="sm"
      data-task-id={task.id}
      className={cx(
        "h-auto min-h-7 w-full justify-start gap-2 rounded-none border-l-2 px-2 py-1 text-left",
        task.color ? swatchClass(task.color, "edge") : "border-transparent",
      )}
      onClick={(e) => (e.metaKey ? openTaskInNewView(task.id) : openTaskInCurrentView(task.id))}
    >
      <span className="min-w-0 flex-1 truncate text-ink">{task.title}</span>
      <StateChip task={task} />
    </Button>
  );
}
