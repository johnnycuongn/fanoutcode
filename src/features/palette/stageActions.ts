// The palette's actions on the focused task, by stage. Every stage action is `tasks_action` with an internally tagged
// StageAction (A1, A17); Reply only focuses the task's terminal (A20).
import type { StageAction, Task } from "@/lib/bindings";

type Simple = Exclude<StageAction["type"], "Back" | "Reply">;
export type StageEntry = { label: string; action: Simple };

export function stageActions(task: Task, mayCommit: boolean): StageEntry[] {
  switch (task.stage) {
    case "Spec":
      return [{ label: "Approve spec", action: "Approve" }];
    case "Plan":
      return [{ label: "Start build", action: "Advance" }];
    case "Build":
      return [{ label: "Request review", action: "RequestReview" }];
    case "Review":
      return [{ label: "Approve changes", action: "ApproveChanges" }];
    case "Test":
      return [
        { label: "Run tests", action: "RunTests" },
        { label: "Fix failing tests", action: "FixTests" },
      ];
    case "Commit":
      return [
        { label: "Copy commit message", action: "CopyCommit" },
        ...(mayCommit ? [{ label: "Commit", action: "Commit" } as StageEntry] : []),
      ];
    case "Done":
      return [];
  }
}
