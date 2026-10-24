import { DndContext, DragOverlay } from "@dnd-kit/core";
import { restrictToWindowEdges } from "@dnd-kit/modifiers";
import { useEffect, useMemo, useRef, useState, type ReactElement } from "react";
import { loadActivity, startActivityEvents } from "../activity";
import { ipc } from "../ipc";
import { all } from "../listen";
import {
  boardStore,
  type BoardState,
  configureBoard,
  loadBoard,
  selectColumns,
  selectMode,
  selectReadOnly,
  selectTotalTaskCount,
  selectVisibleTasks,
  startBoardEvents,
  useBoard,
} from "../store";
import { taskMap, type TaskMap } from "../taskCache";
import type { BoardProps } from "../types";
import { useBoardDnd } from "../useBoardDnd";
import { useBoardShortcuts } from "../useBoardShortcuts";
import { BoardContext, type BoardCtx } from "./boardContext";
import { BoardEmpty } from "./BoardEmpty";
import { BoardHeader } from "./BoardHeader";
import { CardOverlay } from "./BoardCard";
import { ColumnGrid } from "./ColumnGrid";
import { useConfirm } from "./ConfirmDialog";
import { JiraKeyDialog } from "./JiraKeyDialog";
import type { WindowInfo } from "@/lib/bindings";

/** Nothing to show: no tasks at all, or the filters hide every one. Not before the first load finishes. */
function selectEmptyVariant(s: BoardState, tasks: TaskMap): "no_tasks" | "filtered_out" | null {
  if (!s.loaded || selectVisibleTasks(s, tasks).length > 0) return null;
  return selectTotalTaskCount(s, tasks) === 0 && !s.filters.archived ? "no_tasks" : "filtered_out";
}

const containerKey = (c: BoardProps["container"]): string => `${c.type}:${c.id}`;

/** The Kanban board for one window's container. The host passes the props; the board owns everything inside. */
export function Board({
  container,
  projects,
  settings,
  focusedProject,
  currentWindowId,
  onOpenTask,
  onNewTask,
}: BoardProps): ReactElement {
  const rootRef = useRef<HTMLDivElement>(null);
  const latest = useRef({ container, projects });
  const key = containerKey(container);
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [jiraTaskId, setJiraTaskId] = useState<string | null>(null);
  const { confirm, element: confirmElement } = useConfirm();
  const dnd = useBoardDnd();

  useEffect(() => {
    latest.current = { container, projects };
  });
  // The window's container never changes; the key keeps the effect from re-running on a new-but-equal object.
  useEffect(() => {
    const { container: current, projects: initial } = latest.current;
    void loadBoard(current, initial);
    void loadActivity(current);
    return all(startBoardEvents(), startActivityEvents());
  }, [key]);

  useEffect(() => configureBoard({ settings, focusedProject, projects }), [settings, focusedProject, projects]);

  useEffect(() => {
    const refresh = () => void ipc.windowsList().then(setWindows, () => {});
    refresh();
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, []);

  useBoardShortcuts(rootRef, { confirm });

  const ctx = useMemo<BoardCtx>(
    () => ({ windows, currentWindowId, onOpenTask, confirm, openJira: setJiraTaskId }),
    [windows, currentWindowId, onOpenTask, confirm],
  );

  const mode = useBoard(selectMode);
  const readOnly = useBoard(selectReadOnly);
  const columns = useBoard(selectColumns);
  const emptyVariant = useBoard(selectEmptyVariant);
  const jiraTask = jiraTaskId ? taskMap()[jiraTaskId] : undefined;
  const overlayProject = dnd.activeTask ? boardStore.get().projects[dnd.activeTask.project_id] : undefined;

  return (
    <BoardContext.Provider value={ctx}>
      <DndContext
        sensors={dnd.sensors}
        collisionDetection={dnd.collisionDetection}
        modifiers={[restrictToWindowEdges]}
        onDragStart={dnd.onDragStart}
        onDragEnd={dnd.onDragEnd}
        onDragCancel={dnd.onDragCancel}
        accessibility={{ announcements: dnd.announcements }}
      >
        <div
          ref={rootRef}
          data-testid="board"
          data-mode={mode}
          className="flex h-full min-h-0 flex-col bg-page text-ink"
        >
          <BoardHeader />
          {emptyVariant ? (
            <BoardEmpty variant={emptyVariant} onNewTask={onNewTask} />
          ) : (
            <ColumnGrid lane="*" columns={columns} readOnly={readOnly} showProject={false} compactEmpty={false} />
          )}
        </div>
        <DragOverlay dropAnimation={null}>
          {dnd.activeTask ? <CardOverlay task={dnd.activeTask} project={overlayProject} showProject={false} /> : null}
        </DragOverlay>
      </DndContext>
      {confirmElement}
      {jiraTask && <JiraKeyDialog key={jiraTask.id} task={jiraTask} open onClose={() => setJiraTaskId(null)} />}
    </BoardContext.Provider>
  );
}
