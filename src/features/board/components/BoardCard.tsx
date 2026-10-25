import { memo, useCallback, useEffect, useMemo, useState, type ReactElement } from "react";
import type { Project, Task } from "@/lib/bindings";
import { overlapCount, overlapStore } from "@/features/overlap";
import { useActivity } from "../activity";
import { trackTaskCost, untrackTaskCost, useTaskCost } from "../cost";
import { cardMenuItems } from "../menu";
import { selectDetailLevel, setFocusedTask, setRenaming, useBoard } from "../store";
import { useBoardCtx } from "./boardContext";
import { CardMenu, CardMenuButton } from "./CardMenu";
import { DraggableCard } from "./DraggableCard";
import { RenameInline } from "./RenameInline";
import { TaskCard } from "./TaskCard";

type Props = { task: Task; lane: string; readOnly: boolean; showProject: boolean; project?: Project };

/** One card on the board: wires a presentational `TaskCard` to the store, the menu and drag and drop. */
export const BoardCard = memo(function BoardCard({ task, lane, readOnly, showProject, project }: Props): ReactElement {
  const board = useBoardCtx();
  const level = useBoard((s) => selectDetailLevel(s, task.project_id));
  const costOverview = useBoard((s) => s.settings.costOverview);
  const focused = useBoard((s) => s.focusedTaskId === task.id);
  const renaming = useBoard((s) => s.renamingTaskId === task.id);
  const activity = useActivity(task.id);
  const overlaps = overlapStore.useSelector((s) => overlapCount(s, task.id));
  // A card's own cost is shown on Detailed cards and in every card's tooltip (fetched on first hover, not for 200 cards).
  const [hovered, setHovered] = useState(false);
  const wantsCost = costOverview && (level === "Detailed" || hovered);
  useEffect(() => {
    if (!wantsCost) return;
    trackTaskCost(task.id);
    return () => untrackTaskCost(task.id);
  }, [wantsCost, task.id]);
  const cost = useTaskCost(task.id);
  const onHover = useCallback(() => setHovered(true), []);

  const onOpen = useCallback(
    (target: Parameters<typeof board.onOpenTask>[1]) => board.onOpenTask(task.id, target),
    [board, task.id],
  );
  const items = useMemo(
    () =>
      cardMenuItems(task, {
        windows: board.windows,
        currentWindowId: board.currentWindowId,
        onOpen,
        confirm: board.confirm,
        openJira: board.openJira,
      }),
    [task, board, onOpen],
  );
  const menu = useMemo(() => <CardMenuButton items={items} />, [items]);
  const onFocus = useCallback(() => setFocusedTask(task.id), [task.id]);

  return (
    <CardMenu items={items}>
      <DraggableCard task={task} lane={lane} disabled={readOnly || renaming}>
        {({ ref, handleProps, dragging }) => (
          <TaskCard
            ref={ref}
            handleProps={handleProps}
            dragging={dragging}
            task={task}
            level={level}
            activity={activity}
            project={project}
            showProject={showProject}
            costOverview={costOverview}
            cost={costOverview ? cost : null}
            focused={focused}
            overlaps={overlaps}
            disabled={readOnly}
            onHover={onHover}
            onOpen={onOpen}
            onFocus={onFocus}
            menu={menu}
            titleSlot={renaming ? <RenameInline task={task} onDone={() => setRenaming(null)} /> : undefined}
          />
        )}
      </DraggableCard>
    </CardMenu>
  );
});

/** The copy that follows the pointer while a card is dragged. */
export function CardOverlay({ task, project, showProject }: { task: Task; project?: Project; showProject: boolean }) {
  const level = useBoard((s) => selectDetailLevel(s, task.project_id));
  const activity = useActivity(task.id);
  return (
    <TaskCard
      task={task}
      level={level}
      activity={activity}
      project={project}
      showProject={showProject}
      costOverview={false}
      cost={null}
      focused={false}
      dragging={false}
      onOpen={() => {}}
      onFocus={() => {}}
      menu={null}
    />
  );
}
