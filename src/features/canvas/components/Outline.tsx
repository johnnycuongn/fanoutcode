// Outline mode (§10.7): plan.md as a checklist, active steps expanded into their agents' tool and hook stream.
import { useMemo, type ReactElement } from "react";
import { Badge, Checkbox, Chip, cx } from "@/components/ui";
import { asSwatch } from "@/lib/format";
import { MiniMarkdown } from "@/livespec/MiniMarkdown";
import { buildOutline, type OutlineEntry } from "../outline";
import { showRepoBadges } from "../repo";
import { eventsUntil } from "../replay";
import { useCanvas } from "../store";

export function formatDuration(ms: number | null): string {
  if (ms === null) return "";
  return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(1)} s`;
}

function StreamRow({ entry }: { entry: OutlineEntry }): ReactElement {
  return (
    <li data-kind={entry.kind} className={cx("flex gap-2", entry.muted ? "text-mute" : "text-ink")}>
      <span className="min-w-0 flex-1 truncate">{entry.text}</span>
      <span className="shrink-0 text-mute tabular-nums">{formatDuration(entry.durationMs)}</span>
    </li>
  );
}

export function Outline({ taskId }: { taskId: string }): ReactElement {
  const s = useCanvas(taskId);
  const items = useMemo(() => {
    const visible = s.replaySeq === null ? s.events : eventsUntil(s.events, s.replaySeq);
    return buildOutline(s.model, visible);
  }, [s.model, s.events, s.replaySeq]);
  const showRepo = s.task ? showRepoBadges(s.task, s.roots) : false;
  return (
    <ol role="list" aria-label="Plan outline" className="flex h-full flex-col gap-1 overflow-y-auto p-3">
      {items.map(({ step, expanded, stream }) => {
        const root = showRepo ? s.roots.find((r) => r.root === step.repo) : undefined;
        return (
          <li key={step.number} className="flex flex-col gap-1 rounded-card px-2 py-1 hover:bg-well">
            <div className="flex min-w-0 items-center gap-2 text-sm">
              <Checkbox
                size="sm"
                disabled
                checked={step.status === "done"}
                aria-label={`Step ${step.number} ${step.status}`}
              />
              <span className="shrink-0 text-mute tabular-nums">{`${step.number}.`}</span>
              <MiniMarkdown text={step.title} className="min-w-0 flex-1 truncate [&_p]:mb-0 [&_p]:truncate" />
              {step.owners.map((o) => (
                <Chip key={o} size="sm" tone="neutral">
                  {o}
                </Chip>
              ))}
              {root && (
                <Badge size="sm" swatch={asSwatch(root.color)}>
                  {root.name}
                </Badge>
              )}
            </div>
            {expanded && stream.length > 0 && (
              <ul className="ml-7 flex flex-col gap-0.5 border-l border-line pl-3 font-mono text-[11px]">
                {stream.map((e) => (
                  <StreamRow key={`${e.kind}:${e.seq}`} entry={e} />
                ))}
              </ul>
            )}
          </li>
        );
      })}
    </ol>
  );
}
