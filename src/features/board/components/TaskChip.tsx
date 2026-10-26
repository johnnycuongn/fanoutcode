// A card's state chip; a Done task whose files are still uncommitted reads "Not committed".
import type { ReactElement } from "react";
import type { Task } from "@/lib/bindings";
import { useUncommitted } from "@/lib/uncommitted";
import { chipFor } from "../chipText";
import { CardChip } from "./CardChip";

export function TaskChip({ task }: { task: Pick<Task, "id" | "stage" | "state"> }): ReactElement {
  const uncommitted = useUncommitted(task.id, task.stage === "Done");
  const chip = chipFor(task);
  // short enough for a narrow column; a Done task's own state reads Idle, which says nothing here
  return <CardChip model={uncommitted > 0 ? { tone: "needs_you", text: "Not committed" } : chip} />;
}
