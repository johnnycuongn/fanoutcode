// This window's views and their layouts: the focused pane per view, the task picker, and every layout edit. Edits
// are applied locally first (layout.ts) and persisted through `views_update_layout`; structural edits persist at
// once, divider drags after 300 ms. View CRUD, loading and events live in viewsCrud.ts.
import { commands, type Container, type PaneContent, type PaneId, type SplitDirection } from "@/lib/bindings";
import type { TaskId, View, ViewId } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { containerKey } from "@/lib/stores/tasksStore";
import { shellStore } from "@/features/shell";
import {
  EMPTY_PANE,
  leaves,
  movePane,
  neighbour,
  newPaneId,
  removePane,
  sameContent,
  setContent,
  setRatio,
  splitPane,
  taskContent,
  teamCanvasContent,
} from "./layout";
import type { Dir, Edge, LayoutPath } from "./types";

export type ViewsState = {
  container: Container | null;
  views: View[];
  focusedPane: Record<ViewId, PaneId>;
  pickerFor: { viewId: ViewId; pane: PaneId } | null;
  loaded: boolean;
};

export const viewsStore = createStore<ViewsState>({
  container: null,
  views: [],
  focusedPane: {},
  pickerFor: null,
  loaded: false,
});

export const RATIO_DEBOUNCE_MS = 300;

// Views with a local edit not yet saved; a `views-changed` reload keeps their local layout.
export const unsaved = new Set<ViewId>();
const ratioTimers = new Map<ViewId, ReturnType<typeof setTimeout>>();

export const localKey = (c: Container) => `tc:views:${containerKey(c)}`;

/** Remembers the active tab and the focused panes for the relaunch restore. */
export function persistLocal(): void {
  const { container, focusedPane } = viewsStore.get();
  if (!container) return;
  try {
    localStorage.setItem(localKey(container), JSON.stringify({ active: shellStore.get().activeTab, focusedPane }));
  } catch {
    // Storage blocked: the window opens on the Board next time.
  }
}

export function viewById(id: ViewId): View | undefined {
  return viewsStore.get().views.find((v) => v.id === id);
}

export function activeView(): View | null {
  return viewById(shellStore.get().activeTab) ?? null;
}

/** The view's focused pane if it still exists, else its first pane. */
export function focusedPaneOf(view: View, focused = viewsStore.get().focusedPane): PaneId {
  const all = leaves(view.layout);
  const want = focused[view.id];
  return all.some((l) => l.pane === want) ? want : all[0].pane;
}

export function persistLayout(viewId: ViewId): Promise<void> {
  const view = viewById(viewId);
  if (!view) return Promise.resolve();
  unsaved.add(viewId);
  return Promise.resolve(commands.viewsUpdateLayout(viewId, view.layout)).then(
    () => {
      if (viewById(viewId)?.layout === view.layout) unsaved.delete(viewId);
    },
    () => {
      unsaved.delete(viewId);
    },
  );
}

function setLayout(viewId: ViewId, edit: (v: View) => View["layout"]): View | null {
  const view = viewById(viewId);
  if (!view) return null;
  const layout = edit(view);
  if (layout === view.layout) return view;
  const next = { ...view, layout };
  viewsStore.set((s) => ({ ...s, views: s.views.map((v) => (v.id === viewId ? next : v)) }));
  return next;
}

export function focusPane(viewId: ViewId, pane: PaneId): void {
  if (viewsStore.get().focusedPane[viewId] !== pane)
    viewsStore.set((s) => ({ ...s, focusedPane: { ...s.focusedPane, [viewId]: pane } }));
  persistLocal();
}

export function openPicker(viewId: ViewId, pane: PaneId): void {
  viewsStore.set({ pickerFor: { viewId, pane } });
}

export function closePicker(): void {
  if (viewsStore.get().pickerFor) viewsStore.set({ pickerFor: null });
}

/** Splits the view's focused pane, or refocuses a pane already showing `content` (one path for every split). */
export function splitPaneIn(viewId: ViewId, direction: SplitDirection, content: PaneContent): Promise<void> {
  const view = viewById(viewId);
  if (!view) return Promise.resolve();
  if (content.type !== "Empty") {
    const same = leaves(view.layout).find((l) => sameContent(l.content, content));
    if (same) {
      focusPane(viewId, same.pane);
      return Promise.resolve();
    }
  }
  const from = focusedPaneOf(view);
  const pane = newPaneId();
  setLayout(viewId, (v) => splitPane(v.layout, from, direction, pane, content));
  focusPane(viewId, pane);
  viewsStore.set({ pickerFor: content.type === "Empty" ? { viewId, pane } : null });
  return persistLayout(viewId);
}

/** A20: splits the active view's focused pane with `content`, or refocuses the pane that already shows it. */
export function splitFocusedPane(direction: SplitDirection, content: PaneContent): Promise<void> {
  const view = activeView();
  return view ? splitPaneIn(view.id, direction, content) : Promise.resolve();
}

/** ⌘D / ⌘⇧D. */
export function splitFocused(direction: SplitDirection): void {
  void splitFocusedPane(direction, EMPTY_PANE);
}

/** Closes `pane`; focus moves to a neighbour. The view's last pane empties to the picker instead (a view keeps one). */
export function closePane(viewId: ViewId, pane: PaneId): void {
  const view = viewById(viewId);
  if (!view) return;
  const next = (["left", "up", "right", "down"] as Dir[]).map((d) => neighbour(view.layout, pane, d)).find(Boolean);
  const updated = setLayout(viewId, (v) => removePane(v.layout, pane));
  if (!updated) return;
  if (focusedPaneOf(view) === pane) focusPane(viewId, next ?? leaves(updated.layout)[0].pane);
  if (viewsStore.get().pickerFor?.pane === pane && next) closePicker();
  void persistLayout(viewId);
}

/** ⌘W in the active view. */
export function closeFocused(): void {
  const view = activeView();
  if (view) closePane(view.id, focusedPaneOf(view));
}

/** ⌘⌥ arrows; returns the pane now focused, or null when there is none that way. */
export function focusDirection(dir: Dir): PaneId | null {
  const view = activeView();
  if (!view) return null;
  const to = neighbour(view.layout, focusedPaneOf(view), dir);
  if (to) focusPane(view.id, to);
  return to;
}

export function movePaneTo(viewId: ViewId, pane: PaneId, target: PaneId, edge: Edge): void {
  const before = viewById(viewId)?.layout;
  const after = setLayout(viewId, (v) => movePane(v.layout, pane, target, edge));
  if (after && after.layout !== before) void persistLayout(viewId);
}

export function setPaneRatio(viewId: ViewId, path: LayoutPath, ratio: number): void {
  if (!setLayout(viewId, (v) => setRatio(v.layout, path, ratio))) return;
  unsaved.add(viewId);
  clearTimeout(ratioTimers.get(viewId));
  ratioTimers.set(
    viewId,
    setTimeout(() => {
      ratioTimers.delete(viewId);
      void persistLayout(viewId);
    }, RATIO_DEBOUNCE_MS),
  );
}

export function setPaneContent(viewId: ViewId, pane: PaneId, content: PaneContent): void {
  setLayout(viewId, (v) => setContent(v.layout, pane, content));
  if (viewsStore.get().pickerFor?.pane === pane) closePicker();
  void persistLayout(viewId);
}

/** Shows the task in the view's focused pane, replacing what it showed (Plan 04 owns single-attach). */
export function openTaskInView(viewId: ViewId, taskId: TaskId): void {
  const view = viewById(viewId);
  if (!view) return;
  setPaneContent(viewId, focusedPaneOf(view), taskContent(taskId));
}

/** The task page's `onOpenTeamPane`: the team canvas beside `fromPane`, or the existing one refocused. */
export function openTeamPane(viewId: ViewId, fromPane: PaneId, taskId: TaskId): PaneId {
  focusPane(viewId, fromPane);
  void splitPaneIn(viewId, "Horizontal", teamCanvasContent(taskId));
  return viewsStore.get().focusedPane[viewId];
}
