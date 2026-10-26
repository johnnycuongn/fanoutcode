import { useDroppable } from "@dnd-kit/core";
import { memo, useId, type ReactElement } from "react";
import { Badge, cx, stageClasses, type StageToken } from "@/components/ui";
import type { Project, Stage, Task } from "@/lib/bindings";
import { columnDroppableId } from "../dnd";
import { useBoard } from "../store";
import { BoardCard } from "./BoardCard";
import { DoneMoment } from "./DoneMoment";
import { EmptyColumn } from "./EmptyColumn";

export type ColumnProps = {
  stage: Stage;
  lane: string;
  tasks: Task[];
  count: number;
  readOnly: boolean;
  compactEmpty: boolean;
  showProject: boolean;
  projectsById: Record<string, Project>;
};

export const Column = memo(function Column({
  stage,
  lane,
  tasks,
  count,
  readOnly,
  compactEmpty,
  showProject,
  projectsById,
}: ColumnProps): ReactElement {
  const headingId = useId();
  const glow = useBoard((s) => s.lastDrop?.stage === stage && s.lastDrop.lane === lane);
  const doneKey = useBoard((s) => (stage === "Done" && s.lastDrop?.stage === "Done" ? s.lastDrop.at : null));
  const { setNodeRef, isOver } = useDroppable({ id: columnDroppableId(lane, stage), disabled: readOnly });
  return (
    <section
      aria-labelledby={headingId}
      data-stage={stage}
      data-glow={glow}
      className={cx(
        "relative flex min-w-0 basis-0 flex-col rounded-well bg-well/60 p-1.5",
        // an empty column keeps its place and drop target but gives its room to the columns with cards
        tasks.length === 0 ? "grow-[0.6]" : "grow",
        "transition-shadow duration-300 data-[glow=true]:ring-2 data-[glow=true]:ring-accent/40",
      )}
    >
      <header className="mb-1.5 flex items-center gap-1.5 px-1">
        <span
          aria-hidden
          className={cx("size-2 rounded-full bg-current", stageClasses[stage.toLowerCase() as StageToken].text)}
        />
        <h3 id={headingId} className="text-sm font-medium text-ink">
          {stage}
        </h3>
        {stage === "Done" && <DoneMoment playKey={doneKey} />}
        <Badge size="sm" data-testid="column-count" className="ml-auto tabular-nums">
          {count}
        </Badge>
      </header>
      <div
        ref={setNodeRef}
        data-testid="column-body"
        data-over={isOver}
        className="flex min-h-24 flex-1 flex-col gap-1.5 rounded-card data-[over=true]:bg-accent/10"
      >
        {tasks.length === 0 && <EmptyColumn stage={stage} compact={compactEmpty} />}
        {tasks.map((task) => (
          <BoardCard
            key={task.id}
            task={task}
            lane={lane}
            readOnly={readOnly}
            showProject={showProject}
            project={projectsById[task.project_id]}
          />
        ))}
      </div>
    </section>
  );
});
