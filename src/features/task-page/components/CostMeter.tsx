// The task's cost and its context window, shown when the cost overview is on (§10.6).
import type { ReactElement } from "react";
import { cx } from "@/components/ui";
import type { CostSummary } from "@/lib/bindings";
import { formatCost } from "../format";

const tone = (pct: number) => (pct < 70 ? "bg-ok" : pct < 90 ? "bg-warn" : "bg-bad");

export function CostMeter({
  cost,
  contextPct,
}: {
  cost: CostSummary | undefined;
  contextPct: number | undefined;
}): ReactElement | null {
  if (!cost && contextPct === undefined) return null;
  const pct = Math.min(Math.max(contextPct ?? 0, 0), 100);
  return (
    <div className="flex items-center gap-2 text-xs text-mute tabular-nums">
      {cost && <span>{formatCost(cost)}</span>}
      {contextPct !== undefined && (
        <div
          role="meter"
          aria-label="Context window"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(pct)}
          title={`Context ${Math.round(pct)}% used`}
          className="h-1.5 w-12 overflow-hidden rounded-full bg-well"
        >
          <div className={cx("h-full rounded-full", tone(pct))} style={{ width: `${pct}%` }} />
        </div>
      )}
    </div>
  );
}
