import {
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type Announcements,
  type CollisionDetection,
  type DragEndEvent,
  type DragStartEvent,
  type SensorDescriptor,
  type SensorOptions,
} from "@dnd-kit/core";
import { useCallback, useMemo, useState } from "react";
import type { Task } from "@/lib/bindings";
import { moveTask } from "./actions";
import {
  boardAnnouncements,
  boardCollisionDetection,
  boardKeyboardCoordinates,
  handleDragEnd,
  parseLaneId,
} from "./dnd";
import { boardStore, laneOf, reorderLanes, selectReadOnly } from "./store";
import { taskMap } from "./taskCache";

export type BoardDnd = {
  sensors: SensorDescriptor<SensorOptions>[];
  collisionDetection: CollisionDetection;
  onDragStart(event: DragStartEvent): void;
  onDragEnd(event: DragEndEvent): void;
  onDragCancel(): void;
  activeTask: Task | null;
  announcements: Announcements;
};

/** Space picks a card up (Enter stays free to open it), arrows step between columns, Space or Enter drops, Escape cancels. */
const KEYBOARD_CODES = { start: ["Space"], cancel: ["Escape"], end: ["Space", "Enter"] };

function titleOf(id: string): string {
  const lane = parseLaneId(id);
  if (lane !== null) return boardStore.get().projects[lane]?.name ?? "project row";
  return taskMap()[id]?.title ?? "task";
}

export function useBoardDnd(): BoardDnd {
  const sensors = useSensors(
    // A small distance keeps a plain click a click, so it still opens the card.
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: boardKeyboardCoordinates, keyboardCodes: KEYBOARD_CODES }),
  );
  const [activeTask, setActiveTask] = useState<Task | null>(null);

  const onDragStart = useCallback((event: DragStartEvent) => {
    const id = String(event.active.id);
    setActiveTask(parseLaneId(id) === null ? (taskMap()[id] ?? null) : null);
  }, []);

  const onDragEnd = useCallback((event: DragEndEvent) => {
    setActiveTask(null);
    const state = boardStore.get();
    handleDragEnd(event, {
      tasks: taskMap(),
      laneOf: (task) => laneOf(state, task),
      readOnly: selectReadOnly(state),
      laneOrder: state.lanes.order,
      moveTask,
      reorderLanes,
    });
  }, []);

  const onDragCancel = useCallback(() => setActiveTask(null), []);
  const announcements = useMemo(() => boardAnnouncements(titleOf), []);

  return {
    sensors,
    collisionDetection: boardCollisionDetection,
    onDragStart,
    onDragEnd,
    onDragCancel,
    activeTask,
    announcements,
  };
}
