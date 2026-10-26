// The header's primary button for the open task: the stage rule (`primaryActionFor`) fed with the page's settings,
// the queued action, review blockers, and whether the Spec stage's spec.md is still blank.
import type { Task, TaskId } from "@/lib/bindings";
import { specStore } from "@/features/spec-editor";
import { primaryActionFor, type PrimaryButton } from "./primaryAction";
import type { TaskSettings } from "./types";

export function usePrimary(
  task: Task | null | undefined,
  taskId: TaskId,
  settings: TaskSettings,
  queued: boolean,
  blockers: number,
): PrimaryButton | null {
  const spec = specStore.store.useSelector((s) => s.byTask[taskId]);
  const specEmpty = Boolean(spec?.loaded) && !(spec?.text ?? "").trim();
  return task ? primaryActionFor(task, { mayCommit: settings.mayCommit, queued, blockers, specEmpty }) : null;
}
