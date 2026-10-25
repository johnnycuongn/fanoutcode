// By file lens: each file with its owners and net change, its edits in order (`+a −r · step N · role`) with a mini
// diff, then the commands and hook firings the target's agents produced.
import type { ReactElement } from "react";
import { Avatar } from "@/components/ui";
import { formatDuration } from "../format";
import type { ByFileGroup, RollUp } from "../lens";
import { netChange, shortPath } from "./cards/cardParts";

export function ByFileLens({
  groups,
  rollUp,
  roles,
}: {
  groups: ByFileGroup[];
  rollUp: RollUp;
  roles: Record<string, string>;
}): ReactElement {
  return (
    <div className="flex flex-col gap-3 px-3 py-2 text-xs">
      {groups.map((g) => (
        <section key={g.path} aria-label={g.path} className="flex flex-col gap-1">
          <header className="flex items-center gap-2">
            <span className="min-w-0 flex-1 truncate font-mono" title={g.path}>
              {shortPath(g.path)}
            </span>
            <span className="font-mono text-mute">{netChange(g.added, g.removed)}</span>
            <span className="flex -space-x-1">
              {g.owners.map((o) => (
                <Avatar key={o} name={roles[o] ?? "?"} size="xs" />
              ))}
            </span>
          </header>
          {g.edits.map((e) => (
            <div key={e.seq} className="flex flex-col gap-0.5 rounded-well bg-well px-2 py-1">
              <span className="text-mute">
                {[
                  netChange(e.added, e.removed),
                  e.step !== null ? `step ${e.step}` : null,
                  roles[e.agentId] ?? e.agentId,
                ]
                  .filter(Boolean)
                  .join(" · ")}
              </span>
              <pre className="overflow-x-auto font-mono text-[11px]">
                {e.diff.lines.map((l, i) => (
                  <div key={i} className={l.sign === "+" ? "text-ok" : "text-bad"}>{`${l.sign} ${l.text}`}</div>
                ))}
                {e.diff.truncated && <div className="text-mute">…</div>}
              </pre>
            </div>
          ))}
        </section>
      ))}
      {rollUp.commands.length > 0 && (
        <section aria-label="Commands" className="flex flex-col gap-0.5">
          <h3 className="font-medium text-mute">Commands</h3>
          {rollUp.commands.map((c) => (
            <div key={c.preSeq} className="flex gap-2 font-mono">
              <span className="min-w-0 flex-1 truncate">{c.target}</span>
              <span className="text-mute tabular-nums">{formatDuration(c.durationMs)}</span>
            </div>
          ))}
        </section>
      )}
      {rollUp.hooks.length > 0 && (
        <section aria-label="Hook firings" className="flex flex-col gap-0.5 text-mute">
          <h3 className="font-medium">Hooks</h3>
          {rollUp.hooks.map((h) => (
            <div key={h.seq} className="truncate font-mono">
              {h.text}
            </div>
          ))}
        </section>
      )}
    </div>
  );
}
