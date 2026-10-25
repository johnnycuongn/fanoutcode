// Costs at a scope: the total and tokens (Live while a session reports), grouped by project, task, stage or day.
// Plan 06 mounts it in project settings with `{ project }`; the workspace settings with `{ workspace }`.
import { useEffect, type ReactElement } from "react";
import { Chip, EmptyState, Panel, Segmented } from "@/components/ui";
import type { Scope } from "@/lib/bindings";
import { formatTokens, formatUsd, rowsFor, type CostGroup } from "../costFormat";
import { costsStore } from "../costsStore";

export type CostsPanelProps = { scope: Partial<Scope>; names?: Record<string, string> };

const GROUPS: Array<{ value: CostGroup; label: string }> = [
  { value: "project", label: "By project" },
  { value: "task", label: "By task" },
  { value: "stage", label: "By stage" },
  { value: "day", label: "By day" },
];

export function CostsPanel({ scope, names }: CostsPanelProps): ReactElement {
  const { summary, group, enabled } = costsStore.use();
  const key = JSON.stringify(scope);
  useEffect(() => {
    void costsStore.load(JSON.parse(key) as Partial<Scope>).catch(() => undefined);
  }, [key]);
  if (!enabled) return <EmptyState title="Turn on Cost overview in app settings to collect costs." />;
  if (!summary || summary.cost_usd === 0) return <EmptyState title="No cost recorded yet." />;
  const options = scope.project ? GROUPS.filter((g) => g.value !== "project") : GROUPS;
  return (
    <Panel variant="well" className="flex flex-col gap-2">
      <div className="flex items-baseline justify-between gap-2">
        <span className="font-mono text-lg text-ink">{formatUsd(summary.cost_usd, summary.estimated)}</span>
        {summary.live && (
          <Chip size="sm" tone="ok">
            Live
          </Chip>
        )}
        <span className="text-xs text-mute">
          {`${formatTokens(summary.input_tokens)} in · ${formatTokens(summary.output_tokens)} out`}
        </span>
      </div>
      <Segmented
        size="sm"
        aria-label="Group costs"
        options={options}
        value={group}
        onValueChange={costsStore.setGroup}
      />
      <ul className="flex flex-col gap-0.5">
        {rowsFor(summary, group).map((l) => (
          <li key={l.key} className="flex justify-between gap-2 text-sm">
            <span className="truncate text-ink">{names?.[l.key] ?? l.key}</span>
            <span className="shrink-0 font-mono text-mute">
              {`${formatUsd(l.cost_usd, l.estimated)} · ${formatTokens(l.input_tokens)}/${formatTokens(l.output_tokens)}`}
            </span>
          </li>
        ))}
      </ul>
    </Panel>
  );
}
