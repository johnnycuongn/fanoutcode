// A plan step on the canvas (§10.7 "Nodes"): Simple, Medium or Detailed while active; done and upcoming steps
// collapse to their title line. Titles and roles are agent prose, so they go through MiniMarkdown.
import type { ReactElement } from "react";
import { Badge, Chip, StatusDot, cx, type Tone } from "@/components/ui";
import { MiniMarkdown } from "@/livespec/MiniMarkdown";
import type { StepNodeData, ViewOptions } from "../../types";
import { CARD, RepoBadge, costText, netChange, shortPath } from "./cardParts";

const TONE: Record<StepNodeData["status"], Tone> = { done: "ok", active: "accent", upcoming: "neutral" };

function Progress({ value }: { value: number | null }): ReactElement {
  const pct = value === null ? null : Math.round(value * 100);
  return (
    <div
      role="progressbar"
      aria-label="Step progress"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct ?? undefined}
      data-indeterminate={pct === null ? "true" : undefined}
      className="h-1 w-full overflow-hidden rounded-full bg-well"
    >
      <div
        className={cx("h-full rounded-full bg-accent", pct === null && "w-1/3 opacity-50")}
        style={pct === null ? undefined : { width: `${pct}%` }}
      />
    </div>
  );
}

export function StepCard({
  data,
  view,
}: {
  data: StepNodeData & { stackCount?: number };
  view: ViewOptions;
}): ReactElement {
  const collapsed = data.status !== "active";
  const files = data.files;
  return (
    <div
      data-node-kind="step"
      data-status={data.status}
      className={cx(CARD, "flex w-60 flex-col gap-1.5 px-2.5 py-2", data.working ? "border-accent" : "border-line")}
    >
      <div className="flex min-w-0 items-center gap-1.5 text-sm">
        <StatusDot tone={TONE[data.status]} status={data.working ? "working" : undefined} size="sm" />
        <span className="shrink-0 text-mute tabular-nums">{`${data.number}.`}</span>
        <MiniMarkdown text={data.title} className="min-w-0 flex-1 truncate font-medium [&_p]:mb-0 [&_p]:truncate" />
        {view.showRepo && <RepoBadge repo={data.repo} roots={view.roots} />}
        {data.stackIndex === 0 && data.stackCount !== undefined && (
          <Badge size="sm" tone="ok">{`${data.stackCount} done`}</Badge>
        )}
      </div>
      {!collapsed && (
        <>
          <div className="flex flex-wrap items-center gap-1">
            {data.owners.map((o) => (
              <Chip key={o} size="sm" tone="neutral">
                {o}
              </Chip>
            ))}
            <Chip size="sm" tone={data.working ? "accent" : "neutral"}>
              {data.working ? "Working" : "Active"}
            </Chip>
            <span className="ml-auto text-xs text-mute">{`${files.length} ${files.length === 1 ? "file" : "files"}`}</span>
          </div>
          {view.detail !== "Simple" && files.length > 0 && (
            <ul className="flex flex-col gap-0.5 font-mono text-[11px]">
              {files.map((f) => (
                <li key={f.path} className={cx("truncate", f.shared ? "text-warn" : "text-mute")}>
                  {`${shortPath(f.path)} ${netChange(f.added, f.removed)}`}
                </li>
              ))}
            </ul>
          )}
          {view.detail === "Detailed" && (
            <div className="flex flex-col gap-1">
              {data.toolLine && <p className="truncate font-mono text-[11px] text-mute">{data.toolLine}</p>}
              <div className="flex items-center gap-2">
                <Progress value={data.progress} />
                {view.showCost && data.costUsd !== null && (
                  <span className="shrink-0 text-xs text-mute tabular-nums">{costText(data.costUsd)}</span>
                )}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
