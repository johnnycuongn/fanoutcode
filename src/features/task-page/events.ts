// The live events an open task page needs beyond tasksStore's own `task-changed`: the cost meter and the context
// meter, and Plan 05's `spec-watcher` (A15) for the Memory tab's log.
import { events } from "@/lib/bindings";
import { dockStore } from "@/features/dock";
import { taskPageStore, type TaskPageStore } from "./store";

export function subscribeTaskPageEvents(s: TaskPageStore = taskPageStore): () => void {
  const offs = [
    events.costUpdated.listen((e) => {
      if (s.isOpen(e.payload.task_id)) void s.onCostUpdated(e.payload.task_id);
    }),
    events.teamEvent.listen((e) => {
      if (s.isOpen(e.payload.task_id)) s.onTeamEvent(e.payload.task_id, e.payload.event);
    }),
    events.specWatcher.listen((e) => {
      dockStore.addWatcherEntry(e.payload.task_id, { at: e.payload.at, reason: e.payload.reason });
    }),
  ];
  return () => {
    for (const off of offs) void off.then((fn) => fn());
  };
}

let users = 0;
let stop: (() => void) | null = null;

/** One subscription per window, shared by every open task page; returns this page's release. */
export function acquireTaskPageEvents(): () => void {
  if (users++ === 0) stop = subscribeTaskPageEvents();
  let released = false;
  return () => {
    if (released) return;
    released = true;
    if (--users === 0) {
      stop?.();
      stop = null;
    }
  };
}
