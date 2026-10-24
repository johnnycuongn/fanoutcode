import type { ReactElement } from "react";
import { Button, EmptyState, KeyHint } from "@/components/ui";
import boardArt from "../assets/empty-column.lottie?url";
import { clearFilters } from "../store";

/** The board has no tasks at all, or the filters hide every one. */
export function BoardEmpty({
  variant,
  onNewTask,
}: {
  variant: "no_tasks" | "filtered_out";
  onNewTask(): void;
}): ReactElement {
  return (
    <div className="flex flex-1 items-center justify-center">
      {variant === "no_tasks" ? (
        <EmptyState
          size="md"
          animation={boardArt}
          title="No tasks yet"
          description="Describe what you want built, or pull in a Jira issue."
          action={
            <Button variant="primary" onClick={onNewTask} trailing={<KeyHint keys="cmd+t" size="sm" />}>
              New task
            </Button>
          }
        />
      ) : (
        <EmptyState
          size="md"
          title="No cards match these filters"
          action={<Button onClick={clearFilters}>Clear filters</Button>}
        />
      )}
    </div>
  );
}
