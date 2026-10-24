import type { ReactElement } from "react";
import { EmptyState } from "@/components/ui";
import type { Stage } from "@/lib/bindings";
import columnArt from "../assets/empty-column.lottie?url";

export const COLUMN_HINTS: Record<Stage, string> = {
  Spec: "No specs waiting. ⌘T starts a task.",
  Plan: "Approve a spec to start planning.",
  Build: "Press Start build on a planned task.",
  Review: "Request review when a build is done.",
  Test: "Approve changes to run the tests.",
  Commit: "Nothing to commit yet.",
  Done: "Finished tasks land here.",
};

/**
 * An empty column: a small looping illustration and one hint line. `compact` (swimlane cells) drops the illustration, so
 * a workspace board does not run seven animations per project.
 */
export function EmptyColumn({ stage, compact }: { stage: Stage; compact: boolean }): ReactElement {
  const hint = COLUMN_HINTS[stage];
  if (compact) {
    return (
      <p
        data-testid="empty-hint"
        className="rounded-card border border-dashed border-line px-2 py-3 text-center text-xs text-mute"
      >
        {hint}
      </p>
    );
  }
  return (
    <div data-testid="empty-state" className="contents">
      <EmptyState
        size="sm"
        animation={columnArt}
        title={hint}
        className="p-2 [&_h2]:text-xs [&_h2]:font-normal [&_h2]:text-mute"
      />
    </div>
  );
}
