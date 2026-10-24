import type { ReactElement } from "react";
import {
  selectColumns,
  selectMode,
  selectReadOnly,
  selectTotalTaskCount,
  selectVisibleTasks,
  useBoard,
  type BoardState,
} from "../store";
import type { TaskMap } from "../taskCache";
import { BoardEmpty } from "./BoardEmpty";
import { ColumnGrid } from "./ColumnGrid";
import { Swimlanes } from "./Swimlanes";

/** Nothing to show: no tasks at all, or the filters hide every one. Not before the first load finishes. */
function selectEmptyVariant(s: BoardState, tasks: TaskMap): "no_tasks" | "filtered_out" | null {
  if (!s.loaded || selectVisibleTasks(s, tasks).length > 0) return null;
  return selectTotalTaskCount(s, tasks) === 0 && !s.filters.archived ? "no_tasks" : "filtered_out";
}

/** Chooses what fills the board under the header: an empty state, swimlanes, or one grid of columns. */
export function BoardBody({ onNewTask }: { onNewTask(): void }): ReactElement {
  const mode = useBoard(selectMode);
  const readOnly = useBoard(selectReadOnly);
  const picked = useBoard((s) => s.pickedProject);
  const columns = useBoard(selectColumns);
  const empty = useBoard(selectEmptyVariant);

  if (empty) return <BoardEmpty variant={empty} onNewTask={onNewTask} />;
  if (mode === "swimlanes") return <Swimlanes readOnly={false} />;
  if (mode === "one_project" && picked === null) return <Swimlanes readOnly />;
  const lane = mode === "one_project" && picked !== null ? picked : "*";
  return (
    <ColumnGrid lane={lane} columns={columns} readOnly={readOnly} showProject={mode === "mixed"} compactEmpty={false} />
  );
}
