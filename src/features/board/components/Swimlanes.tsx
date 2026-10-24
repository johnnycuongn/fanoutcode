import { SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import type { ReactElement } from "react";
import { laneDraggableId } from "../dnd";
import { selectSwimlanes, useBoard } from "../store";
import { SwimlaneRow } from "./SwimlaneRow";

/** The workspace board as rows, one per project. `readOnly` (One project, All) hides the grips and blocks drops. */
export function Swimlanes({ readOnly }: { readOnly: boolean }): ReactElement {
  const lanes = useBoard(selectSwimlanes);
  const focusedProject = useBoard((s) => s.focusedProject);
  return (
    <SortableContext items={lanes.map((l) => laneDraggableId(l.projectId))} strategy={verticalListSortingStrategy}>
      <div role="table" aria-label="Projects" data-testid="swimlanes" className="flex-1 overflow-auto p-3">
        <div className="min-w-[1400px]">
          {lanes.map((lane) => (
            <SwimlaneRow
              key={lane.projectId}
              lane={lane}
              readOnly={readOnly}
              focused={focusedProject === lane.projectId}
            />
          ))}
        </div>
      </div>
    </SortableContext>
  );
}
