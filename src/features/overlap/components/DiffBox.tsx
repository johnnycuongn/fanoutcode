// One side of the decision pane: a task's changes to the file against its base (`git_diff` with that task's id),
// headed "<Who>'s changes · +a −r".
import { useEffect, useMemo, useState, type ReactElement } from "react";
import { DiffText } from "@/components/ui";
import { commands, type FileDiff, type TaskId } from "@/lib/bindings";
import { diffStats, lineDiff } from "@/lib/lineDiff";

export interface DiffBoxProps {
  who: string;
  taskId: TaskId;
  repo: string;
  path: string;
}

type Loaded = { key: string; diff: FileDiff | null; error: string | null };

export function DiffBox({ who, taskId, repo, path }: DiffBoxProps): ReactElement {
  const key = `${taskId}\u0000${repo}\u0000${path}`;
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  useEffect(() => {
    let live = true;
    void commands.gitDiff(taskId, repo, path).then((r) => {
      if (!live) return;
      setLoaded(r.status === "ok" ? { key, diff: r.data, error: null } : { key, diff: null, error: r.error });
    });
    return () => {
      live = false;
    };
  }, [key, taskId, repo, path]);
  const current = loaded?.key === key ? loaded : null;
  const diff = current?.diff ?? null;
  const stats = useMemo(() => (diff ? diffStats(lineDiff(diff.before, diff.after)) : null), [diff]);
  const title = stats ? `${who}'s changes · +${stats.added} −${stats.removed}` : `${who}'s changes`;
  return (
    <div className="grid min-w-0 content-start gap-1">
      <h4 className="text-xs font-semibold text-ink">{title}</h4>
      {!current && <p className="text-xs text-mute">Loading the diff…</p>}
      {current?.error && <p className="text-xs text-bad">{current.error}</p>}
      {diff?.binary && <p className="text-xs text-mute">A binary file; no text diff.</p>}
      {diff && !diff.binary && <DiffText aria-label={title} before={diff.before} after={diff.after} />}
    </div>
  );
}
