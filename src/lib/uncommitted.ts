// How many of a task's files are still uncommitted (`git_uncommitted`, the Commit stage's own Done check), kept fresh
// on `git-changes`. "Mark done" asks first, and a Done task says so on its card and header.
import { useEffect, useState } from "react";
import { commands, events, type TaskId } from "@/lib/bindings";

/** 0 when unknown: a count on a card must never break the card. */
export async function uncommittedCount(taskId: TaskId): Promise<number> {
  try {
    const r = await commands.gitUncommitted(taskId);
    return r?.status === "ok" ? r.data : 0;
  } catch {
    return 0;
  }
}

/** The task's uncommitted file count while `enabled`; 0 otherwise or until known. */
export function useUncommitted(taskId: TaskId, enabled: boolean): number {
  const [count, setCount] = useState(0);
  useEffect(() => {
    if (!enabled) return;
    let live = true;
    const load = () => void uncommittedCount(taskId).then((n) => live && setCount(n));
    load();
    const off = events.gitChanges
      .listen((e) => {
        if (e.payload.task_id === taskId) load();
      })
      .catch(() => () => {});
    return () => {
      live = false;
      void off.then((fn) => fn());
    };
  }, [taskId, enabled]);
  return enabled ? count : 0;
}
