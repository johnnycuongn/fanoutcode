// How many agents are working across every project, for the start screen's spinning brand mark. Fed by
// `tasks_working_count` and refreshed on `task-changed` and `tasks-reloaded`.
import { useEffect } from "react";
import { commands, events } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";

export const workingStore = createStore<{ count: number }>({ count: 0 });

// Each load takes a ticket; only the newest one's answer is kept, so a slow early reply never overwrites a later one.
let ticket = 0;

export async function loadWorkingCount(): Promise<void> {
  const mine = ++ticket;
  const r = await commands.tasksWorkingCount();
  if (mine === ticket && r?.status === "ok") workingStore.set({ count: r.data });
}

/** Loads the count and keeps it fresh on `task-changed` and `tasks-reloaded`; returns the unsubscribe. */
export function startWorkingEvents(): () => void {
  void loadWorkingCount();
  const offs = [
    events.taskChanged.listen(() => void loadWorkingCount()),
    events.tasksReloaded.listen(() => void loadWorkingCount()),
  ];
  return () => {
    for (const off of offs) void off.then((fn) => fn());
  };
}

/** True while any agent in the app is working. */
export function useAgentsWorking(): boolean {
  useEffect(() => startWorkingEvents(), []);
  return workingStore.useSelector((s) => s.count > 0);
}
