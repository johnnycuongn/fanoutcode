// The other side of the decision pane (§5.4): what this session was about to do, from `overlap_pending_input`. The
// tool input can lag the stop by a moment, so a missing one shows a loading line and is fetched again on every
// `overlap-changed` for the task.
import { useEffect, useState, type ReactElement } from "react";
import { DiffText } from "@/components/ui";
import { commands, events, type PendingDecision, type TaskId } from "@/lib/bindings";
import { needsCurrentFile, willWrite, type WillWrite } from "../willWrite";

export interface WillWriteBoxProps {
  taskTitle: string;
  taskId: TaskId;
  repo: string;
  path: string;
}

async function load(taskId: TaskId, repo: string, path: string): Promise<WillWrite | null> {
  const r = await commands.overlapPendingInput(taskId, repo, path);
  const pending: PendingDecision | null = r.status === "ok" ? r.data : null;
  let current: string | null = null;
  if (needsCurrentFile(pending)) {
    const f = await commands.gitFileRead(taskId, repo, path);
    // A file that does not exist yet is an empty "before".
    current = f.status === "ok" ? f.data : "";
  }
  return willWrite(pending, current);
}

export function WillWriteBox({ taskTitle, taskId, repo, path }: WillWriteBoxProps): ReactElement {
  const key = `${taskId}\u0000${repo}\u0000${path}`;
  const [shown, setShown] = useState<{ key: string; what: WillWrite } | null>(null);
  useEffect(() => {
    let live = true;
    const refetch = () =>
      void load(taskId, repo, path).then((what) => {
        if (live && what) setShown({ key, what });
      });
    refetch();
    const off = events.overlapChanged.listen((e) => {
      if (e.payload.task_id === taskId) refetch();
    });
    return () => {
      live = false;
      void off.then((fn) => fn());
    };
  }, [key, taskId, repo, path]);
  const what = shown?.key === key ? shown.what : null;
  const title = `What ${taskTitle} will write`;
  return (
    <div className="grid min-w-0 content-start gap-1">
      <h4 className="text-xs font-semibold text-ink">{title}</h4>
      {!what && <p className="text-xs text-mute">{`Loading what ${taskTitle} will write…`}</p>}
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
