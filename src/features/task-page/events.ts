// The live events an open task page needs beyond tasksStore's own `task-changed`: the cost meter and the context
// meter. Task 19 adds Plan 05's `spec-watcher` (A15).
import { events } from "@/lib/bindings";
import { taskPageStore, type TaskPageStore } from "./store";

export function subscribeTaskPageEvents(s: TaskPageStore = taskPageStore): () => void {
  const offs = [
    events.costUpdated.listen((e) => {
      if (s.isOpen(e.payload.task_id)) void s.onCostUpdated(e.payload.task_id);
    }),
    events.teamEvent.listen((e) => {
      if (s.isOpen(e.payload.task_id)) s.onTeamEvent(e.payload.task_id, e.payload.event);
    }),
  ];
  return () => {
    for (const off of offs) void off.then((fn) => fn());
  };
}
