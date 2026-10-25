// The Overlap dock tab (§7): the overlapping files on the left, the selected file's decision pane on the right. The
// first pending file is selected until the developer picks another. Above them, a stuck merge's hand-back pane and, on a
// Done worktree task, Bring <base> in (§6.3).
import { useEffect, useState, type ReactElement } from "react";
import { Button, EmptyState } from "@/components/ui";
import type { OverlapRow, Project, Task } from "@/lib/bindings";
import { overlapStore, refreshOverlaps } from "../store";
import { rowKey } from "../labels";
import { DecisionPane } from "./DecisionPane";
import { IntegratePane } from "./IntegratePane";
import { OverlapFileList } from "./OverlapFileList";
import { bringBaseInLabel, canBringBaseIn, useBringBaseIn } from "./useBringBaseIn";

export interface OverlapTabProps {
  task: Task;
  projects: Project[];
}

const NO_ROWS: OverlapRow[] = [];

function OverlapFiles({ task, projects }: OverlapTabProps): ReactElement {
  const loaded = overlapStore.useSelector((s) => s.byTask[task.id]);
  const rows = loaded ?? NO_ROWS;
  const [picked, setPicked] = useState<string | null>(null);
  useEffect(() => {
    if (!loaded) void refreshOverlaps(task.id);
  }, [loaded, task.id]);
  if (rows.length === 0) return <EmptyState size="sm" title="No other task is touching these files" />;
  const ordered = [...rows.filter((r) => r.pending !== null), ...rows.filter((r) => r.pending === null)];
  const row = ordered.find((r) => rowKey(r) === picked) ?? ordered[0];
  const key = rowKey(row);
  return (
    <div className="grid min-h-0 flex-1 grid-cols-[minmax(12rem,18rem)_1fr]">
      <div className="min-h-0 border-r border-line">
        <OverlapFileList rows={rows} selected={key} onSelect={setPicked} projects={projects} />
      </div>
      <div className="min-h-0 overflow-auto">
        <DecisionPane key={`${key}\u0000${row.pending ? "pending" : "settled"}`} task={task} row={row} />
      </div>
    </div>
  );
}

export function OverlapTab({ task, projects }: OverlapTabProps): ReactElement {
  const bringBaseIn = useBringBaseIn(task);
  return (
    <div className="flex h-full min-h-0 flex-col">
      <IntegratePane task={task} />
      {canBringBaseIn(task) && (
        <div className="flex items-center justify-between gap-2 border-b border-line p-3">
          <p className="text-xs text-mute">
            The work is committed. Merge the base into this branch, keeping both sides.
          </p>
          <Button size="sm" variant="primary" onClick={bringBaseIn.start}>
            {bringBaseInLabel(task)}
          </Button>
        </div>
      )}
      <OverlapFiles task={task} projects={projects} />
      {bringBaseIn.dialog}
    </div>
  );
}
