// The Overlap dock tab (§7): the overlapping files on the left, the selected file's decision pane on the right. The
// first pending file is selected until the developer picks another.
import { useEffect, useState, type ReactElement } from "react";
import { EmptyState } from "@/components/ui";
import type { OverlapRow, Project, Task } from "@/lib/bindings";
import { overlapStore, refreshOverlaps } from "../store";
import { rowKey } from "../labels";
import { DecisionPane } from "./DecisionPane";
import { OverlapFileList } from "./OverlapFileList";

export interface OverlapTabProps {
  task: Task;
  projects: Project[];
}

const NO_ROWS: OverlapRow[] = [];

export function OverlapTab({ task, projects }: OverlapTabProps): ReactElement {
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
    <div className="grid h-full min-h-0 grid-cols-[minmax(12rem,18rem)_1fr]">
      <div className="min-h-0 border-r border-line">
        <OverlapFileList rows={rows} selected={key} onSelect={setPicked} projects={projects} />
      </div>
      <div className="min-h-0 overflow-auto">
        <DecisionPane key={`${key}\u0000${row.pending ? "pending" : "settled"}`} task={task} row={row} />
      </div>
    </div>
  );
}
