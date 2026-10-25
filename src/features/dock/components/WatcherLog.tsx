// What the spec watcher changed in spec.md, newest first (A15), fed by the `spec-watcher` event.
import type { ReactElement } from "react";
import type { TaskId } from "@/lib/bindings";
import { dockStore, type DockStore } from "../store";

const EMPTY: never[] = [];

function hhmm(at: string): string {
  const d = new Date(at);
  if (Number.isNaN(d.getTime())) return at;
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

export function WatcherLog({ taskId, store = dockStore }: { taskId: TaskId; store?: DockStore }): ReactElement | null {
  const entries = store.store.useSelector((s) => s.watcherLog[taskId] ?? EMPTY);
  if (entries.length === 0) return null;
  return (
    <section className="mt-4 border-t border-line pt-3">
      <h3 className="mb-1 text-[11px] font-semibold tracking-wider text-mute uppercase">Spec watcher</h3>
      <ul className="grid gap-0.5 text-xs text-mute">
        {entries.map((e, i) => (
          <li key={`${e.at}-${i}`}>
            <span className="tabular-nums">{hhmm(e.at)}</span> · {e.reason}
          </li>
        ))}
      </ul>
    </section>
  );
}
