// The active view's cost: the sum of `cost_summary` over the tasks it shows (`~` when any part is estimated),
// refetched per task on `cost-updated`.
import { useEffect, useState, type ReactElement } from "react";
import { Badge } from "@/components/ui";
import { events, type TaskId, type ViewId } from "@/lib/bindings";
import { formatUsd } from "@/lib/format";
import { costSummary, type CostSummary } from "@/lib/pendingCommands";
import { taskScope } from "@/features/settings";
import { viewTaskIds, viewsStore } from "@/features/views";

function useTaskCosts(ids: TaskId[]): Record<TaskId, CostSummary> {
  const [byTask, setByTask] = useState<Record<TaskId, CostSummary>>({});
  const key = ids.join(",");
  useEffect(() => {
    let live = true;
    const wanted = new Set(key ? key.split(",") : []);
    const fetchOne = async (id: TaskId) => {
      const r = await costSummary(taskScope(id));
      if (live && r?.status === "ok") setByTask((s) => ({ ...s, [id]: r.data }));
    };
    for (const id of wanted) void fetchOne(id);
    const off = events.costUpdated.listen((e) => {
      if (wanted.has(e.payload.task_id)) void fetchOne(e.payload.task_id);
    });
    return () => {
      live = false;
      void Promise.resolve(off).then((fn) => fn?.());
    };
  }, [key]);
  return byTask;
}

export function ViewCostBadge({ viewId }: { viewId: ViewId }): ReactElement | null {
  const view = viewsStore.useSelector((s) => s.views.find((v) => v.id === viewId));
  const ids = view ? viewTaskIds(view) : [];
  const costs = useTaskCosts(ids);
  const shown = ids.map((id) => costs[id]).filter((c): c is CostSummary => Boolean(c));
  if (shown.length === 0) return null;
  const total = shown.reduce((sum, c) => sum + c.cost_usd, 0);
  const estimated = shown.some((c) => c.estimated);
  return (
    <Badge size="sm" mono title="Cost of the tasks in this view">
      {formatUsd(total, estimated)}
    </Badge>
  );
}
