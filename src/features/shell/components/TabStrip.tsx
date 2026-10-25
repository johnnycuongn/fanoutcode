// The window's tab strip: Board is tab zero, then one tab per view (drag to reorder), then New view.
import { DndContext, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { horizontalListSortingStrategy, SortableContext, arrayMove } from "@dnd-kit/sortable";
import { Plus, SquareKanban } from "lucide-react";
import { useEffect, type ReactElement } from "react";
import { IconButton, Tabs, TabsList, TabsTrigger } from "@/components/ui";
import { registerHandler } from "@/lib/keyboard";
import { projectsStore } from "@/lib/stores/projectsStore";
import { tasksStore } from "@/lib/stores/tasksStore";
import {
  createView,
  reorderViews,
  tabNeedsYouCount,
  viewProjectIds,
  viewStage,
  viewStatus,
  viewsStore,
} from "@/features/views";
import { cycleTab, setActiveTab, shellStore } from "../store";
import { TabItem } from "./TabItem";

export const BOARD_PANEL_ID = "tc-board";

function useTabKeys() {
  useEffect(() => {
    const offs = [
      registerHandler("board", () => setActiveTab("board")),
      registerHandler("view.prev", () => cycleTab(-1)),
      registerHandler("view.next", () => cycleTab(1)),
      registerHandler("view.new", () => void createView()),
    ];
    return () => offs.forEach((off) => off());
  }, []);
}

export function TabStrip(): ReactElement {
  const activeTab = shellStore.useSelector((s) => s.activeTab);
  const workspace = shellStore.useSelector((s) => s.kind === "workspace");
  const views = viewsStore.useSelector((s) => s.views);
  const tasksById = tasksStore.useSelector((s) => s.byId);
  const projectsById = projectsStore.useSelector((s) => s.byId);
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));
  useTabKeys();

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    const ids = views.map((v) => v.id);
    void reorderViews(arrayMove(ids, ids.indexOf(String(active.id)), ids.indexOf(String(over.id))));
  };

  return (
    <div className="flex min-w-0 items-end bg-bar">
      <Tabs value={activeTab} onValueChange={setActiveTab} activationMode="manual" className="min-w-0 overflow-x-auto">
        <TabsList variant="bar" aria-label="Views">
          <TabsTrigger
            value="board"
            aria-controls={BOARD_PANEL_ID}
            icon={<SquareKanban aria-hidden className="size-4" />}
            onClick={() => setActiveTab("board")}
          >
            Board
          </TabsTrigger>
          <DndContext sensors={sensors} onDragEnd={onDragEnd}>
            <SortableContext items={views.map((v) => v.id)} strategy={horizontalListSortingStrategy}>
              {views.map((view) => (
                <TabItem
                  key={view.id}
                  view={view}
                  active={view.id === activeTab}
                  count={tabNeedsYouCount(view, tasksById)}
                  status={viewStatus(view, tasksById)}
                  stage={viewStage(view, tasksById)}
                  colors={
                    workspace ? viewProjectIds(view, tasksById).map((id) => projectsById[id]?.color ?? "slate") : []
                  }
                />
              ))}
            </SortableContext>
          </DndContext>
        </TabsList>
      </Tabs>
      <IconButton
        label="New view"
        size="sm"
        className="mb-0.5 ml-1"
        icon={<Plus aria-hidden className="size-3.5" />}
        onClick={() => void createView()}
      />
    </div>
  );
}
