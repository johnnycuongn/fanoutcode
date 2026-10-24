// Opening a task from the sidebar, the attention strip, the palette or a notification: into the active view's
// focused pane (or the pane already showing it), or into the first view when the Board is showing, creating
// `View 1` when there is none.
import type { TaskId } from "@/lib/bindings";
import { createView, focusPane, leaves, openTaskInView, viewById } from "@/features/views";
import { requestDockTab, setActiveTab, shellStore } from "./store";
import type { DockTab } from "./types";

export function openTaskInCurrentView(taskId: TaskId, opts: { dockTab?: DockTab } = {}): void {
  if (opts.dockTab) requestDockTab(taskId, opts.dockTab);
  const { activeTab, tabOrder } = shellStore.get();
  const viewId = activeTab !== "board" ? activeTab : tabOrder[1];
  if (!viewId) {
    void createView("View 1").then((id) => id && openTaskInView(id, taskId));
    return;
  }
  setActiveTab(viewId);
  const view = viewById(viewId);
  const showing = view && leaves(view.layout).find((l) => l.content.type === "Task" && l.content.task_id === taskId);
  if (showing) focusPane(viewId, showing.pane);
  else openTaskInView(viewId, taskId);
}

/** ⌘-click: the task in a new view of its own. */
export function openTaskInNewView(taskId: TaskId, opts: { dockTab?: DockTab } = {}): void {
  if (opts.dockTab) requestDockTab(taskId, opts.dockTab);
  void createView().then((id) => id && openTaskInView(id, taskId));
}
