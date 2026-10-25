// The Overlap tab's left column (§7): the files, grouped "Needs you" (a decision is pending) and "Also touched by
// others", each with the other task, its stage and state, and the relation.
import type { ReactElement } from "react";
import { Badge, StatusDot, cx } from "@/components/ui";
import type { OverlapRow, Project, Task, TaskId } from "@/lib/bindings";
import { agentStatus, stageToken } from "@/lib/format";
import { tasksStore } from "@/lib/stores/tasksStore";
import { otherTitle, relationLabel, rowKey } from "../labels";

export interface OverlapFileListProps {
  rows: OverlapRow[];
  selected: string | null;
  onSelect(key: string): void;
  projects: Project[];
}

function repoLabel(root: string, projects: Project[]): string {
  return projects.find((p) => p.root === root)?.name ?? root.split("/").filter(Boolean).pop() ?? root;
}

function Row(p: {
  row: OverlapRow;
  other: Task | undefined;
  byId: Record<TaskId, Task>;
  repo: string | null;
  selected: boolean;
  onSelect(): void;
}): ReactElement {
  const { row, other } = p;
  return (
    <li
      role="option"
      aria-selected={p.selected}
      tabIndex={0}
      onClick={p.onSelect}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          p.onSelect();
        }
      }}
      className={cx(
        "grid cursor-pointer gap-0.5 rounded px-2 py-1 text-xs outline-none hover:bg-well",
        "focus-visible:ring-2 focus-visible:ring-accent/60",
        p.selected && "bg-well text-ink",
      )}
    >
      <span className="flex min-w-0 items-center gap-1.5">
        {p.repo && (
          <Badge size="sm" mono>
            {p.repo}
          </Badge>
        )}
        <span className="min-w-0 flex-1 truncate font-mono" title={row.path}>
          {row.path}
        </span>
      </span>
      <span className="flex min-w-0 items-center gap-1.5 text-mute">
        {other && <StatusDot size="sm" status={agentStatus(other.state)} />}
        <span className="min-w-0 truncate">{otherTitle(row.other_task, p.byId)}</span>
        {other && (
          <Badge size="sm" stage={stageToken(other.stage)}>
            {other.stage}
          </Badge>
        )}
        <Badge size="sm" tone={row.held ? "warn" : row.relation === "TakenOver" ? "bad" : "neutral"}>
          {relationLabel(row)}
        </Badge>
      </span>
    </li>
  );
}

function Group(p: { title: string; rows: OverlapRow[]; props: OverlapFileListProps; byId: Record<TaskId, Task> }) {
  if (p.rows.length === 0) return null;
  const multiRepo = p.props.projects.length > 1;
  return (
    <section className="mb-3">
      <h3 className="px-2 py-1 text-xs font-semibold text-ink">{p.title}</h3>
      <ul role="listbox" aria-label={p.title} className="grid gap-px">
        {p.rows.map((row) => {
          const key = rowKey(row);
          return (
            <Row
              key={key}
              row={row}
              other={row.other_task ? p.byId[row.other_task] : undefined}
              byId={p.byId}
              repo={multiRepo ? repoLabel(row.repo_root, p.props.projects) : null}
              selected={p.props.selected === key}
              onSelect={() => p.props.onSelect(key)}
            />
          );
        })}
      </ul>
    </section>
  );
}

export function OverlapFileList(props: OverlapFileListProps): ReactElement {
  const byId = tasksStore.useSelector((s) => s.byId);
  const pending = props.rows.filter((r) => r.pending !== null);
  const rest = props.rows.filter((r) => r.pending === null);
  return (
    <nav aria-label="Overlapping files" className="h-full overflow-auto py-2">
      <Group title="Needs you" rows={pending} props={props} byId={byId} />
      <Group title="Also touched by others" rows={rest} props={props} byId={byId} />
    </nav>
  );
}
