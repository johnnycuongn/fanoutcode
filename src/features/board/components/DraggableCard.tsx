import { useDraggable } from "@dnd-kit/core";
import { useCallback, useMemo, type ReactNode, type Ref } from "react";
import type { Task } from "@/lib/bindings";

export type DraggableRender = { ref: Ref<HTMLElement>; handleProps: Record<string, unknown>; dragging: boolean };

/** Makes a card draggable by id; the render prop receives what `TaskCard` needs (ref, dnd attributes and listeners). */
export function DraggableCard({
  task,
  lane,
  disabled,
  children,
}: {
  task: Task;
  lane: string;
  disabled: boolean;
  children(p: DraggableRender): ReactNode;
}) {
  const { setNodeRef, setActivatorNodeRef, attributes, listeners, isDragging } = useDraggable({
    id: task.id,
    disabled,
    data: { lane, stage: task.stage },
  });
  // The card is both the node and the activator, so keystrokes typed inside it (the rename box) never start a drag.
  const ref = useCallback(
    (el: HTMLElement | null) => {
      setNodeRef(el);
      setActivatorNodeRef(el);
    },
    [setNodeRef, setActivatorNodeRef],
  );
  const handleProps = useMemo(() => {
    const props: Record<string, unknown> = { ...attributes, ...listeners };
    delete props["aria-pressed"]; // the card is an article, not a toggle button
    return props;
  }, [attributes, listeners]);
  return <>{children({ ref, handleProps, dragging: isDragging })}</>;
}
