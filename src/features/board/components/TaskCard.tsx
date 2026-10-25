import { Bookmark, Flag } from "lucide-react";
import { memo, type KeyboardEvent, type MouseEvent, type ReactElement, type ReactNode, type Ref } from "react";
import { Badge, Tooltip, cx } from "@/components/ui";
import type { AgentKind, CostSummary, DetailLevel, Priority, Project, Task } from "@/lib/bindings";
import { swatchClass } from "@/lib/theme";
import { overlapsText } from "@/features/overlap";
import { chipFor } from "../chipText";
import { formatCost, formatTokens } from "../cost";
import type { CardActivity, OpenTarget } from "../types";
import { CardChip } from "./CardChip";
import { CardStepper } from "./CardStepper";
import { CardTeamRow } from "./CardTeamRow";
import { ElapsedTime } from "./ElapsedTime";

export type TaskCardProps = {
  task: Task;
  level: DetailLevel;
  activity: CardActivity;
  project?: Project;
  showProject: boolean;
  costOverview: boolean;
  cost: CostSummary | null;
  focused: boolean;
  dragging?: boolean;
  /** Read-only boards (One project, All) keep the card openable but not draggable. */
  disabled?: boolean;
  onOpen(target: OpenTarget): void;
  onFocus(): void;
  /** The pointer entered the card (cost is fetched lazily for the tooltip). */
  onHover?(): void;
  /** The `…` button; clicks inside it never open the task. */
  menu: ReactNode;
  /** How many files this task shares with other tasks (§7); a badge shows when above zero. */
  overlaps?: number;
  /** Replaces the title while renaming. */
  titleSlot?: ReactNode;
  ref?: Ref<HTMLElement>;
  /** dnd-kit's attributes and listeners. */
  handleProps?: Record<string, unknown>;
};

const AGENTS: Record<AgentKind, { initials: "CC" | "CX" | "SH"; name: string }> = {
  ClaudeCode: { initials: "CC", name: "Claude Code" },
  Codex: { initials: "CX", name: "Codex" },
  Shell: { initials: "SH", name: "Shell" },
};

export const agentInitials = (kind: AgentKind): "CC" | "CX" | "SH" => AGENTS[kind].initials;

const FLAG_TONE: Record<Exclude<Priority, "None">, string> = {
  High: "text-warn",
  Medium: "text-ink",
  Low: "text-mute",
};

const basename = (path: string): string => path.slice(path.lastIndexOf("/") + 1);

function CardTooltip({ activity, cost }: { activity: CardActivity; cost: CostSummary | null }): ReactElement {
  return (
    <div className="flex flex-col gap-0.5">
      <span>{activity.lastLine ?? "No agent output yet"}</span>
      {cost && (
        <span className="text-page/70">
          {formatCost(cost)} · {formatTokens(cost.input_tokens + cost.output_tokens)} tokens
        </span>
      )}
    </div>
  );
}

function TaskCardImpl(props: TaskCardProps): ReactElement {
  const { task, level, activity, project, showProject, costOverview, cost, focused, dragging, disabled } = props;
  const { onOpen, onFocus, onHover, menu, titleSlot, ref, handleProps, overlaps = 0 } = props;
  const { onKeyDown: dndKeyDown, ...handle } = (handleProps ?? {}) as { onKeyDown?: (e: KeyboardEvent) => void };
  const agent = AGENTS[task.agent_kind];
  const colour = task.color;

  const open = (e: MouseEvent) => {
    if (!dragging) onOpen(e.metaKey ? { kind: "new_view" } : { kind: "current" });
  };
  const keyDown = (e: KeyboardEvent<HTMLElement>) => {
    dndKeyDown?.(e);
    // While a keyboard drag is on, Enter drops the card; it must not also open it.
    if (e.key === "Enter" && !dragging && !e.defaultPrevented && e.target === e.currentTarget) {
      onOpen({ kind: "current" });
    }
  };

  return (
    <Tooltip content={<CardTooltip activity={activity} cost={costOverview ? cost : null} />} side="right" delay={600}>
      <article
        {...handle}
        ref={ref}
        role="article"
        tabIndex={0}
        data-task-id={task.id}
        data-level={level}
        data-hidden={task.hidden}
        data-focused={focused}
        data-dragging={dragging ?? false}
        aria-disabled={disabled || undefined}
        onClick={open}
        onKeyDown={keyDown}
        onFocus={onFocus}
        onPointerEnter={onHover}
        className={cx(
          "group flex cursor-pointer flex-col gap-1.5 rounded-card border border-line bg-card p-2 text-sm text-ink outline-none",
          "focus-visible:ring-2 focus-visible:ring-accent/60 data-[focused=true]:ring-1 data-[focused=true]:ring-accent",
          "data-[hidden=true]:opacity-60 data-[dragging=true]:opacity-40",
          colour && cx("border-l-2", swatchClass(colour, "edge"), "border-t-line border-r-line border-b-line"),
        )}
      >
        <div className="flex items-center gap-1.5">
          {task.priority !== "None" && (
            <Flag
              role="img"
              aria-label={`Priority ${task.priority}`}
              className={cx("size-3.5 shrink-0", FLAG_TONE[task.priority])}
            />
          )}
          {task.flagged && <Bookmark role="img" aria-label="Flagged" className="size-3.5 shrink-0 text-accent" />}
          <span className="min-w-0 flex-1 truncate font-medium">{titleSlot ?? task.title}</span>
          {task.jira_key && (
            <Badge size="sm" mono>
              {task.jira_key}
            </Badge>
          )}
          {showProject && project && (
            <Badge size="sm" className={cx(swatchClass(project.color, "bg"), "text-accent-ink")}>
              {project.name}
            </Badge>
          )}
          {task.archived && <Badge size="sm">Archived</Badge>}
          <div onClick={(e) => e.stopPropagation()} className="shrink-0">
            {menu}
          </div>
        </div>
        <div className="flex items-center gap-1.5 text-xs">
          <CardChip model={chipFor(task)} />
          {overlaps > 0 && (
            <Badge size="sm" className="shrink-0">
              {overlapsText(overlaps)}
            </Badge>
          )}
          <span
            role="img"
            aria-label={agent.name}
            className="inline-flex size-5 shrink-0 items-center justify-center rounded-full bg-well text-[9px] font-medium text-mute"
          >
            {agent.initials}
          </span>
          <span className="ml-auto">
            <ElapsedTime since={task.state_entered_at} />
          </span>
        </div>
        {level !== "Simple" && (
          <>
            <CardStepper stage={task.stage} />
            {activity.currentFile && (
              <div data-testid="current-file" className="flex items-center gap-1.5 font-mono text-xs">
                <span className="min-w-0 flex-1 truncate" title={activity.currentFile.path}>
                  {basename(activity.currentFile.path)}
                </span>
                <span className="text-ok">+{activity.currentFile.added}</span>
                <span className="text-bad">−{activity.currentFile.removed}</span>
              </div>
            )}
          </>
        )}
        {level === "Detailed" && (
          <div className="flex items-center gap-2">
            <CardTeamRow team={activity.team} />
            {costOverview && cost && <span className="ml-auto text-xs text-mute tabular-nums">{formatCost(cost)}</span>}
          </div>
        )}
      </article>
    </Tooltip>
  );
}

/** Memoised: it never reads the clock (ElapsedTime does), so a card re-renders only when its own props change. */
export const TaskCard = memo(TaskCardImpl);
