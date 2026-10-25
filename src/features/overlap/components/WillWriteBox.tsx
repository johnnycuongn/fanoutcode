// The other side of the decision pane (§5.4): what this session was about to do, from `overlap_pending_input`. The
// tool input can lag the stop by a moment, so a missing one shows a loading line (after 10 s, a line saying it has
// not landed yet) and is fetched again on every `overlap-changed` for the task. An `Edit` is labelled with the lines
// its old string falls on in the current file.
import { useEffect, useState, type ReactElement } from "react";
import { DiffText } from "@/components/ui";
import { commands, events, type PendingDecision, type TaskId } from "@/lib/bindings";
import { editLines, needsCurrentFile, willWrite, type WillWrite } from "../willWrite";

/** How long the loading line shows before it says the tool call has not landed yet. */
const SLOW_MS = 10_000;

interface Shown {
  what: WillWrite;
  lines: string | null;
}

export interface WillWriteBoxProps {
  taskTitle: string;
  taskId: TaskId;
  /** The row's other task (`null`: another session). */
  otherTask: TaskId | null;
  repo: string;
  path: string;
}

async function load(taskId: TaskId, otherTask: TaskId | null, repo: string, path: string): Promise<Shown | null> {
  const r = await commands.overlapPendingInput(taskId, repo, path, otherTask);
  const pending: PendingDecision | null = r.status === "ok" ? r.data : null;
  let current: string | null = null;
  if (needsCurrentFile(pending)) {
    const f = await commands.gitFileRead(taskId, repo, path);
    // A file that does not exist yet is an empty "before".
    current = f.status === "ok" ? f.data : "";
  }
  const what = willWrite(pending, current);
  return what && { what, lines: editLines(pending, current) };
}

export function WillWriteBox({ taskTitle, taskId, otherTask, repo, path }: WillWriteBoxProps): ReactElement {
  const key = `${taskId}\u0000${otherTask ?? ""}\u0000${repo}\u0000${path}`;
  const [shown, setShown] = useState<(Shown & { key: string }) | null>(null);
  const [slowKey, setSlowKey] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    const refetch = () =>
      void load(taskId, otherTask, repo, path).then((got) => {
        if (live && got) setShown({ key, ...got });
      });
    refetch();
    const slow = setTimeout(() => setSlowKey(key), SLOW_MS);
    const off = events.overlapChanged.listen((e) => {
      if (e.payload.task_id === taskId) refetch();
    });
    return () => {
      live = false;
      clearTimeout(slow);
      void off.then((fn) => fn());
    };
  }, [key, taskId, otherTask, repo, path]);
  const current = shown?.key === key ? shown : null;
  const what = current?.what ?? null;
  const waiting = slowKey === key ? "The tool call hasn't landed yet; it shows here when it does." : null;
  const title = `What ${taskTitle} will write`;
  return (
    <div className="grid min-w-0 content-start gap-1">
      <h4 className="text-xs font-semibold text-ink">{title}</h4>
      {!what && <p className="text-xs text-mute">{waiting ?? `Loading what ${taskTitle} will write…`}</p>}
      {current?.lines && <p className="text-xs text-mute">{current.lines}</p>}
      {what?.type === "Diff" && <DiffText aria-label={title} before={what.before} after={what.after} />}
      {what?.type === "Command" && (
        <pre className="overflow-auto rounded-well border border-line bg-well px-2 py-1.5 font-mono text-xs text-ink">
          {what.command}
        </pre>
      )}
      {what?.type === "Raw" && (
        <pre className="overflow-auto rounded-well border border-line bg-well px-2 py-1.5 font-mono text-xs text-mute">
          {what.text}
        </pre>
      )}
    </div>
  );
}
