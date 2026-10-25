// Replaying a finished Build (§10.7): the canvas is derived from a prefix of the durable event log.
import type { Task, TaskEvent } from "@/lib/bindings";

/** The events up to and including `seq`; `events` is already normalised (ascending, unique seqs). */
export function eventsUntil(events: TaskEvent[], seq: number): TaskEvent[] {
  let lo = 0;
  let hi = events.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (events[mid].seq <= seq) lo = mid + 1;
    else hi = mid;
  }
  return lo === events.length ? events : events.slice(0, lo);
}

export function replayBounds(
  events: TaskEvent[],
): { firstSeq: number; lastSeq: number; startAt: string; endAt: string } | null {
  if (events.length === 0) return null;
  const first = events[0];
  const last = events[events.length - 1];
  return { firstSeq: first.seq, lastSeq: last.seq, startAt: first.at, endAt: last.at };
}

/** The greatest `seq` whose time is at or before `atMs`, else the first seq. */
export function seqAtTime(events: TaskEvent[], atMs: number): number {
  let found = events[0]?.seq ?? 0;
  for (const e of events) {
    if (Date.parse(e.at) > atMs) break;
    found = e.seq;
  }
  return found;
}

export function canReplay(task: Pick<Task, "state">, events: TaskEvent[]): boolean {
  return events.length > 0 && task.state.type !== "Working";
}
