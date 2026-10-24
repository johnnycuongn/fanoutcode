// One view: its LayoutNode rendered recursively, pane moves by dragging a header onto another pane's edge, and the
// view's chords while it is the active tab.
import { DndContext, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { useState, type ReactElement } from "react";
import type { ViewId } from "@/lib/bindings";
import { shellStore } from "@/features/shell";
import { focusedPaneOf, movePaneTo, viewsStore } from "../store";
import type { Edge } from "../types";
import { useViewKeys } from "../useViewKeys";
import { SplitNode } from "./SplitNode";

export function ViewCanvas({ viewId }: { viewId: ViewId }): ReactElement | null {
  const view = viewsStore.useSelector((s) => s.views.find((v) => v.id === viewId));
  const focused = viewsStore.useSelector((s) => s.focusedPane[viewId]);
  const active = shellStore.useSelector((s) => s.activeTab === viewId);
  const [dragging, setDragging] = useState(false);
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));
  useViewKeys(viewId, active);
  if (!view) return null;
  const focusedPane = focusedPaneOf(view, { [viewId]: focused });

  const onDragEnd = (e: DragEndEvent) => {
    setDragging(false);
    const target = e.over?.data.current as { target: string; edge: Edge } | undefined;
    if (target) movePaneTo(viewId, String(e.active.id), target.target, target.edge);
  };

  return (
    <DndContext
      sensors={sensors}
      onDragStart={() => setDragging(true)}
      onDragEnd={onDragEnd}
      onDragCancel={() => setDragging(false)}
    >
      <div data-view-id={viewId} className="h-full min-h-0 w-full min-w-0 bg-page">
        <SplitNode viewId={viewId} node={view.layout} path={[]} focusedPane={focusedPane} dragging={dragging} />
      </div>
    </DndContext>
  );
}
