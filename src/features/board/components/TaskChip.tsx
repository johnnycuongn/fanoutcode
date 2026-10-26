// A card's state chip; a Done task whose files are still uncommitted says so instead of a plain "Done".
import type { ReactElement } from "react";
import type { Task } from "@/lib/bindings";
import { useUncommitted } from "@/lib/uncommitted";
import { chipFor } from "../chipText";
import { CardChip } from "./CardChip";

export function TaskChip({ task }: { task: Pick<Task, "id" | "stage" | "state"> }): ReactElement {
  const uncommitted = useUncommitted(task.id, task.stage === "Done");
  const chip = chipFor(task);
  return <CardChip model={uncommitted > 0 ? { ...chip, tone: "needs_you", detail: "not committed" } : chip} />;
}
