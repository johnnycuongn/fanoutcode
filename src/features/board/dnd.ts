// Drag and drop as pure functions: droppable ids, where a drop lands, keyboard stepping between columns, announcements.
// The dnd-kit hooks that use them are in useBoardDnd.ts.
import {
  closestCenter,
  pointerWithin,
  type Announcements,
  type CollisionDetection,
  type DragEndEvent,
  type KeyboardCoordinateGetter,
} from "@dnd-kit/core";
import type { Stage, Task } from "@/lib/bindings";
import { STAGES } from "./columns";
import type { DropOutcome } from "./types";

type Rect = { left: number; top: number; width: number; height: number };
type Column = { lane: string; stage: Stage };

// ---- ids -----------------------------------------------------------------------------------------------------------

export const columnDroppableId = (lane: string, stage: Stage): string => `col:${lane}:${stage}`;
export const laneDraggableId = (projectId: string): string => `lane:${projectId}`;

export function parseDroppableId(id: string): Column | null {
  if (!id.startsWith("col:")) return null;
  const cut = id.lastIndexOf(":");
  const lane = id.slice(4, cut);
  const stage = id.slice(cut + 1) as Stage;
  return lane && STAGES.includes(stage) ? { lane, stage } : null;
}

export function parseLaneId(id: string): string | null {
  return id.startsWith("lane:") && id.length > 5 ? id.slice(5) : null;
}

// ---- where a drop lands --------------------------------------------------------------------------------------------

export function resolveDrop(input: {
  task: (Pick<Task, "stage" | "project_id"> & { id: string }) | undefined;
  overId: string | null;
  laneOf: (task: Task) => string;
  readOnly: boolean;
}): DropOutcome {
  const { task, overId } = input;
  const target = overId === null ? null : parseDroppableId(overId);
  if (!task || !target) return { kind: "none" }; // the task vanished mid-drag, or the drop was on nothing
  if (input.readOnly) return { kind: "rejected", reason: "read_only" };
  if (input.laneOf(task as Task) !== target.lane) return { kind: "rejected", reason: "cross_lane" };
  if (task.stage === target.stage) return { kind: "rejected", reason: "same_column" };
  return { kind: "move", taskId: task.id, from: task.stage, to: target.stage };
}

export function handleDragEnd(
  event: Pick<DragEndEvent, "active" | "over">,
  deps: {
    tasks: Record<string, Task>;
    laneOf: (task: Task) => string;
    readOnly: boolean;
    laneOrder: string[];
    moveTask(taskId: string, to: Stage): Promise<void>;
    reorderLanes(from: number, to: number): void;
  },
): DropOutcome {
  const activeId = String(event.active.id);
  const lane = parseLaneId(activeId);
  if (lane !== null) {
    const over = event.over ? parseLaneId(String(event.over.id)) : null;
    if (over !== null && over !== lane) deps.reorderLanes(deps.laneOrder.indexOf(lane), deps.laneOrder.indexOf(over));
    return { kind: "none" };
  }
  const outcome = resolveDrop({
    task: deps.tasks[activeId],
    overId: event.over ? String(event.over.id) : null,
    laneOf: deps.laneOf,
    readOnly: deps.readOnly,
  });
  if (outcome.kind === "move") void deps.moveTask(outcome.taskId, outcome.to);
  return outcome;
}

// ---- keyboard ------------------------------------------------------------------------------------------------------

/** The centre of the neighbouring column in the same lane, or undefined at the edge or where that column is absent. */
export function nextColumnTarget(
  direction: "left" | "right",
  current: Column,
  rects: Map<string, Rect>,
): { x: number; y: number } | undefined {
  const index = STAGES.indexOf(current.stage) + (direction === "right" ? 1 : -1);
  const stage = STAGES[index];
  const rect = stage ? rects.get(columnDroppableId(current.lane, stage)) : undefined;
  return rect ? { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 } : undefined;
}

/** Arrow keys step the dragged card to the next or previous column; any other key does not move it. */
export const boardKeyboardCoordinates: KeyboardCoordinateGetter = (event, { context }) => {
  const direction = event.code === "ArrowRight" ? "right" : event.code === "ArrowLeft" ? "left" : null;
  if (!direction) return undefined;
  event.preventDefault();
  const { active, over, droppableRects, collisionRect } = context;
  if (!active || !collisionRect) return undefined;
  const data = active.data.current as { lane?: string; stage?: Stage } | undefined;
  const here =
    (over && parseDroppableId(String(over.id))) ||
    (data?.lane && data.stage ? { lane: data.lane, stage: data.stage } : null);
  if (!here) return undefined;
  const rects = new Map<string, Rect>();
  droppableRects.forEach((rect, id) => rects.set(String(id), rect));
  const target = nextColumnTarget(direction, here, rects);
  // The getter returns where the dragged rectangle's top-left should go, so centre it on the target column.
  return target ? { x: target.x - collisionRect.width / 2, y: target.y - collisionRect.height / 2 } : undefined;
};

// ---- collisions and announcements ----------------------------------------------------------------------------------

/** Cards collide with columns, lanes with lanes; the pointer decides, the dragged rectangle is the fallback. */
export const boardCollisionDetection: CollisionDetection = (args) => {
  const draggingLane = parseLaneId(String(args.active.id)) !== null;
  const droppableContainers = args.droppableContainers.filter((c) =>
    draggingLane ? parseLaneId(String(c.id)) !== null : parseDroppableId(String(c.id)) !== null,
  );
  const narrowed = { ...args, droppableContainers };
  if (draggingLane) return closestCenter(narrowed);
  const hits = pointerWithin(narrowed);
  return hits.length > 0 ? hits : closestCenter(narrowed);
};

export function boardAnnouncements(titleOf: (id: string) => string): Announcements {
  const stageOf = (id: string | number | undefined): string => parseDroppableId(String(id ?? ""))?.stage ?? "";
  return {
    onDragStart: ({ active }) =>
      `Picked up ${titleOf(String(active.id))} from ${(active.data.current as { stage?: string } | undefined)?.stage ?? "its column"}`,
    onDragOver: ({ active, over }) =>
      stageOf(over?.id)
        ? `${titleOf(String(active.id))} is over ${stageOf(over?.id)}`
        : `${titleOf(String(active.id))} is not over a column`,
    onDragEnd: ({ active, over }) =>
      stageOf(over?.id)
        ? `${titleOf(String(active.id))} dropped in ${stageOf(over?.id)}`
        : `${titleOf(String(active.id))} was dropped`,
    onDragCancel: ({ active }) => `Dragging ${titleOf(String(active.id))} cancelled`,
  };
}
