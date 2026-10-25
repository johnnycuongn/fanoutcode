// The Team tab (§10.7): Canvas or Outline over the task's team, Tidy, Replay for a finished Build, the ⌘⇧J hint,
// the drill-in panel and the replay scrubber. The dock's Team tab and the team's own pane both render it.
import { useEffect, type ReactElement } from "react";
import { Button, EmptyState, KeyHint, Segmented, Tooltip, cx } from "@/components/ui";
import type { TeamPlacement } from "@/lib/bindings";
import { degradationNotice } from "../placement";
import { canReplay } from "../replay";
import { closeCanvas, openCanvas, setViewMode, startReplay, tidy, useCanvas } from "../store";
import type { TeamViewMode } from "../types";
import { DrillIn } from "./DrillIn";
import { Outline } from "./Outline";
import { ReplayScrubber } from "./ReplayScrubber";
import { TeamCanvas } from "./TeamCanvas";

const SHORTCUT_EFFECT: Record<TeamPlacement, string> = {
  DockGrows: "Grow the dock to two thirds",
  Swap: "Swap the terminal and the team",
  OwnPane: "Open the team in its own pane",
};

const MODES = [
  { value: "canvas", label: "Canvas" },
  { value: "outline", label: "Outline" },
] as const;

export function TeamTab({ taskId, variant = "dock" }: { taskId: string; variant?: "dock" | "pane" }): ReactElement {
  const s = useCanvas(taskId);
  useEffect(() => {
    void openCanvas(taskId);
    return () => closeCanvas(taskId);
  }, [taskId]);

  const notice = degradationNotice(s.events);
  let body: ReactElement;
  if (!s.task)
    body = (
      <EmptyState size="sm" className="h-full justify-center" title={s.loading ? "Loading team…" : "Task not found"} />
    );
  else if (s.task.agent_kind !== "ClaudeCode")
    body = <EmptyState size="sm" className="h-full justify-center" title="Team view needs Claude Code hooks" />;
  else if (s.steps.length === 0 && s.events.length === 0)
    body = (
      <EmptyState
        size="sm"
        className="h-full justify-center"
        title="Nothing to show yet — the canvas fills as the plan is written and the build runs"
      />
    );
  else
    body = (
      <div className="relative min-h-0 flex-1">
        {s.viewMode === "canvas" ? <TeamCanvas taskId={taskId} /> : <Outline taskId={taskId} />}
        <DrillIn taskId={taskId} />
      </div>
    );

  return (
    <div
      data-team-tab={taskId}
      data-variant={variant}
      className={cx("flex h-full min-h-0 flex-col bg-page", variant === "pane" && "border-l border-line")}
    >
      <div className="flex items-center gap-2 border-b border-line bg-bar px-3 py-1.5">
        <Segmented<TeamViewMode>
          aria-label="Team view"
          size="sm"
          value={s.viewMode}
          onValueChange={(m) => setViewMode(taskId, m)}
          options={MODES}
        />
        <Button size="sm" variant="ghost" onClick={() => void tidy(taskId)}>
          Tidy
        </Button>
        {s.task && canReplay(s.task, s.events) && s.replaySeq === null && (
          <Button size="sm" variant="ghost" onClick={() => startReplay(taskId)}>
            Replay
          </Button>
        )}
        <span className="ml-auto" />
        <Tooltip content={SHORTCUT_EFFECT[s.settings.placement]}>
          <KeyHint keys="cmd+shift+j" size="sm" tabIndex={0} />
        </Tooltip>
      </div>
      {notice && <p className="border-b border-line bg-warn/10 px-3 py-1 text-xs text-warn">{notice}</p>}
      <div className="flex min-h-0 flex-1 flex-col">{body}</div>
      <ReplayScrubber taskId={taskId} />
    </div>
  );
}
