import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { ChevronDown, ChevronRight, GripVertical } from "lucide-react";
import { useCallback, useEffect, useRef, type ReactElement } from "react";
import { Badge, IconButton, cx } from "@/components/ui";
import { swatchClass } from "@/lib/theme";
import { laneDraggableId } from "../dnd";
import { toggleLane } from "../store";
import type { Swimlane } from "../types";
import { ColumnGrid } from "./ColumnGrid";

/** One project's row in a workspace board: a sticky header, then the project's own seven columns. */
export function SwimlaneRow({
  lane,
  readOnly,
  focused,
}: {
  lane: Swimlane;
  readOnly: boolean;
  focused: boolean;
}): ReactElement {
  const { setNodeRef, setActivatorNodeRef, attributes, listeners, transform, transition, isDragging } = useSortable({
    id: laneDraggableId(lane.projectId),
    disabled: readOnly,
  });
  const row = useRef<HTMLDivElement | null>(null);
  const ref = useCallback(
    (el: HTMLDivElement | null) => {
      row.current = el;
      setNodeRef(el);
    },
    [setNodeRef],
  );
  useEffect(() => {
    if (focused) row.current?.scrollIntoView({ block: "nearest" });
  }, [focused]);

  const Chevron = lane.collapsed ? ChevronRight : ChevronDown;
  return (
    <div
      ref={ref}
      role="row"
      data-project={lane.projectId}
      data-focused={focused}
      data-dragging={isDragging}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={cx(
        "mb-3 flex items-stretch gap-2 rounded-well border border-line bg-card/40",
        "data-[focused=true]:ring-1 data-[focused=true]:ring-accent data-[dragging=true]:opacity-60",
      )}
    >
      <div
        role="rowheader"
        className="sticky left-0 z-10 flex w-40 shrink-0 flex-col gap-1.5 rounded-l-well bg-bar p-2"
      >
        <span
          data-testid="lane-color"
          aria-hidden
          className={cx("h-1 w-full rounded-full", swatchClass(lane.color, "bg"))}
        />
        <div className="flex items-center gap-1">
          <span className="min-w-0 flex-1 truncate font-medium text-ink" title={lane.name}>
            {lane.name}
          </span>
          {lane.needsYou > 0 && (
            <Badge size="sm" tone="warn" data-testid="lane-needs-you">
              {lane.needsYou}
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-0.5">
          <IconButton
            size="sm"
            label={`${lane.collapsed ? "Expand" : "Collapse"} ${lane.name}`}
            icon={<Chevron aria-hidden className="size-3.5" />}
            onClick={() => toggleLane(lane.projectId)}
          />
          {!readOnly && (
            <IconButton
              size="sm"
              label={`Reorder ${lane.name}`}
              icon={<GripVertical aria-hidden className="size-3.5" />}
              ref={setActivatorNodeRef}
              {...attributes}
              {...listeners}
            />
          )}
        </div>
      </div>
      {!lane.collapsed && (
        <div role="cell" className="min-w-0 flex-1 py-2 pr-2">
          <ColumnGrid
            lane={lane.projectId}
            columns={lane.columns}
            readOnly={readOnly}
            showProject={false}
            compactEmpty
            scroller={false}
          />
        </div>
      )}
    </div>
  );
}
