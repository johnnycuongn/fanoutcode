// Cost at a scope: the total (with `~` when estimated), tokens, and the lines by task, stage and day. Plan 10 reuses
// it for the workspace panel.
import { useEffect, useState, type ReactElement } from "react";
import { Panel } from "@/components/ui";
import { events, type Scope } from "@/lib/bindings";
import { formatUsd } from "@/lib/format";
import { costSummary, type CostLine, type CostSummary } from "@/lib/pendingCommands";
import { tasksStore } from "@/lib/stores/tasksStore";

function useCostSummary(scope: Scope): CostSummary | null {
  const [summary, setSummary] = useState<CostSummary | null>(null);
  const key = JSON.stringify(scope);
  useEffect(() => {
    let live = true;
    const s = JSON.parse(key) as Scope;
    const load = () => void costSummary(s).then((r) => live && r?.status === "ok" && setSummary(r.data));
    load();
    const off = events.costUpdated.listen(() => load());
    return () => {
      live = false;
      void Promise.resolve(off).then((fn) => fn?.());
    };
  }, [key]);
  return summary;
}

function Lines({ title, lines, label }: { title: string; lines: CostLine[]; label(key: string): string }) {
  if (lines.length === 0) return null;
  return (
    <div>
      <h4 className="py-1 text-xs font-medium text-mute">{title}</h4>
      <ul className="flex flex-col gap-0.5">
        {lines.map((l) => (
          <li key={l.key} className="flex justify-between gap-2 text-sm">
            <span className="truncate text-ink">{label(l.key)}</span>
            <span className="shrink-0 font-mono text-mute">{formatUsd(l.cost_usd, l.estimated)}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

const capitalise = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

export function CostsPanel({ scope }: { scope: Scope }): ReactElement | null {
  const summary = useCostSummary(scope);
  const titles = tasksStore.useSelector((s) => s.byId);
  if (!summary) return null;
  return (
    <Panel variant="well" className="flex flex-col gap-2">
      <div className="flex items-baseline justify-between">
        <span className="font-mono text-lg text-ink">{formatUsd(summary.cost_usd, summary.estimated)}</span>
        <span className="text-xs text-mute">{`${summary.input_tokens} in · ${summary.output_tokens} out`}</span>
      </div>
      <Lines title="By task" lines={summary.by_task} label={(k) => titles[k]?.title ?? k} />
      <Lines title="By stage" lines={summary.by_stage} label={capitalise} />
      <Lines title="By day" lines={summary.by_day} label={(k) => k} />
    </Panel>
  );
}
