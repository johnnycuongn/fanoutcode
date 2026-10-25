// Event stream lens: newest first, `time · tool · target · result · duration`, hook rows muted.
import type { ReactElement } from "react";
import { cx } from "@/components/ui";
import { clockTime, formatDuration } from "../format";
import type { StreamRow } from "../lens";

const RESULT_CLASS = { ok: "text-ok", error: "text-bad", pending: "text-warn" } as const;

export function EventStreamLens({ rows }: { rows: StreamRow[] }): ReactElement {
  if (rows.length === 0) return <p className="px-3 py-2 text-sm text-mute">No events match the filters.</p>;
  return (
    <div role="table" aria-label="Event stream" className="flex flex-col font-mono text-[11px]">
      {rows.map((r) => (
        <div
          key={`${r.kind}:${r.seq}`}
          role="row"
          data-kind={r.kind}
          className={cx("flex items-center gap-2 border-b border-line px-3 py-1", r.muted && "text-mute")}
        >
          <span role="cell" className="shrink-0 tabular-nums text-mute">
            {clockTime(r.at)}
          </span>
          <span role="cell" className="w-14 shrink-0 truncate">
            {r.tool ?? "hook"}
          </span>
          <span role="cell" className="min-w-0 flex-1 truncate">
            {r.target}
          </span>
          <span role="cell" className={cx("shrink-0", r.result && RESULT_CLASS[r.result])}>
            {r.result ?? ""}
          </span>
          <span role="cell" className="w-12 shrink-0 text-right tabular-nums text-mute">
            {formatDuration(r.durationMs)}
          </span>
        </div>
      ))}
    </div>
  );
}
