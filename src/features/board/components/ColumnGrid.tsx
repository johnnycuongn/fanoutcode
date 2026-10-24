import type { ReactElement } from "react";
import { cx } from "@/components/ui";
import { useBoard } from "../store";
import type { BoardColumn } from "../types";
import { Column } from "./Column";

/** Seven columns side by side. Narrower than 1,200 px the row scrolls sideways rather than squeezing. */
export function ColumnGrid({
  lane,
  columns,
  readOnly,
  showProject,
  compactEmpty,
  scroller = true,
}: {
  lane: string;
  columns: BoardColumn[];
  readOnly: boolean;
  showProject: boolean;
  compactEmpty: boolean;
  /** False inside swimlanes, where the table owns the one scrollbar. */
  scroller?: boolean;
}): ReactElement {
  const projectsById = useBoard((s) => s.projects);
  const grid = (
    <div data-testid="column-grid" className="flex min-w-[1200px] gap-2">
      {columns.map((column) => (
        <Column
          key={column.stage}
          stage={column.stage}
          lane={lane}
          tasks={column.tasks}
          count={column.count}
          readOnly={readOnly}
          compactEmpty={compactEmpty}
          showProject={showProject}
          projectsById={projectsById}
        />
      ))}
    </div>
  );
  if (!scroller) return grid;
  return (
    <div data-testid="column-scroller" className={cx("flex-1 overflow-x-auto overflow-y-auto p-3")}>
      {grid}
    </div>
  );
}
