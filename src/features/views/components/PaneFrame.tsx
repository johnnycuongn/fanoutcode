// One pane: a header (drag handle, title, close), the pane's content, and four edge drop zones while a pane is
// being dragged. Clicking or focusing anything inside makes it the view's focused pane.
import { useDraggable, useDroppable } from "@dnd-kit/core";
import { GripVertical, X } from "lucide-react";
import type { ReactElement } from "react";
import { cx, IconButton } from "@/components/ui";
import type { PaneContent, PaneId, ViewId } from "@/lib/bindings";
import { tasksStore } from "@/lib/stores/tasksStore";
import { closePane, focusPane } from "../store";
import type { Edge } from "../types";
import { PaneContentHost } from "./PaneContentHost";

export type PaneFrameProps = {
  viewId: ViewId;
  pane: PaneId;
  content: PaneContent;
  focused: boolean;
  dragging?: boolean;
};

function usePaneTitle(content: PaneContent): string {
  const taskTitle = tasksStore.useSelector((s) =>
    content.type === "Task" || content.type === "TeamCanvas" ? s.byId[content.task_id]?.title : undefined,
  );
  if (content.type === "Empty") return "Empty pane";
  if (content.type === "Terminal") return "Terminal";
  if (content.type === "TeamCanvas") return `Team · ${taskTitle ?? "task"}`;
  return taskTitle ?? "Task";
}

const EDGE_CLASS: Record<Edge, string> = {
  left: "inset-y-0 left-0 w-1/4",
  right: "inset-y-0 right-0 w-1/4",
  top: "inset-x-0 top-0 h-1/4",
  bottom: "inset-x-0 bottom-0 h-1/4",
};

function EdgeZone({ pane, edge }: { pane: PaneId; edge: Edge }): ReactElement {
  const { setNodeRef, isOver } = useDroppable({ id: `${pane}:${edge}`, data: { target: pane, edge } });
  return (
    <div
      ref={setNodeRef}
      data-drop-edge={edge}
      className={cx("absolute z-10 rounded-card", EDGE_CLASS[edge], isOver ? "bg-accent/25" : "bg-transparent")}
    />
  );
}

export function PaneFrame({ viewId, pane, content, focused, dragging = false }: PaneFrameProps): ReactElement {
  const title = usePaneTitle(content);
  const { setNodeRef, listeners, attributes, isDragging } = useDraggable({ id: pane });
  return (
    <div
      data-pane-id={pane}
      data-focused={focused ? "true" : "false"}
      data-scope="pane"
      onClick={() => focusPane(viewId, pane)}
      onFocusCapture={() => focusPane(viewId, pane)}
      className={cx(
        "relative flex h-full min-h-0 min-w-0 flex-col border",
        focused ? "border-accent/60" : "border-transparent",
        isDragging && "opacity-60",
      )}
    >
      <div className="flex h-7 shrink-0 items-center gap-1 border-b border-line bg-bar px-1 text-xs text-mute">
        <span
          ref={setNodeRef}
          {...listeners}
          {...attributes}
          aria-label={`Move pane ${title}`}
          className="flex size-5 cursor-grab items-center justify-center rounded-card hover:bg-well"
        >
          <GripVertical aria-hidden className="size-3.5" />
        </span>
        <span className={cx("min-w-0 flex-1 truncate", focused && "text-ink")} title={title}>
          {title}
        </span>
        <IconButton
          label="Close pane"
          size="sm"
          icon={<X aria-hidden className="size-3" />}
          onClick={(e) => {
            e.stopPropagation();
            closePane(viewId, pane);
          }}
        />
      </div>
      <div className="min-h-0 flex-1">
        <PaneContentHost viewId={viewId} pane={pane} content={content} focused={focused} />
      </div>
      {dragging &&
        !isDragging &&
        (["left", "right", "top", "bottom"] as Edge[]).map((edge) => <EdgeZone key={edge} pane={pane} edge={edge} />)}
    </div>
  );
}
