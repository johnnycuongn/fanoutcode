// Loading, creating, renaming, reordering and deleting views, the relaunch restore, and the `views-changed` reload.
import { commands, events, type Container, type PaneId, type View, type ViewId } from "@/lib/bindings";
import { sameContainer } from "@/lib/stores/tasksStore";
import { setActiveTab, setTabOrder, shellStore } from "@/features/shell";
import { leaves } from "./layout";
import { localKey, openPicker, persistLocal, unsaved, viewsStore, focusPane } from "./store";

const byOrder = (a: View, b: View) => a.order - b.order;

function readLocal(c: Container): { active?: string; focusedPane?: Record<ViewId, PaneId> } {
  try {
    return JSON.parse(localStorage.getItem(localKey(c)) ?? "{}");
  } catch {
    return {};
  }
}

function syncTabs(views: View[]) {
  setTabOrder(views.map((v) => v.id));
}

/** Loads the container's views into the tabs and restores the active tab and focused panes from the last session. */
export async function loadViews(container: Container): Promise<void> {
  const r = await commands.viewsList(container);
  if (r?.status !== "ok") return;
  const views = [...r.data].sort(byOrder);
  const saved = readLocal(container);
  viewsStore.set({ container, views, focusedPane: { ...saved.focusedPane }, pickerFor: null, loaded: true });
  syncTabs(views);
  if (saved.active && views.some((v) => v.id === saved.active)) setActiveTab(saved.active);
}

async function reload(container: Container) {
  const r = await commands.viewsList(container);
  if (r?.status !== "ok") return;
  const local = viewsStore.get();
  const views = [...r.data].sort(byOrder).map((v) => {
    const mine = local.views.find((l) => l.id === v.id);
    return mine && unsaved.has(v.id) ? { ...v, layout: mine.layout } : v;
  });
  const focusedPane: Record<ViewId, PaneId> = {};
  for (const v of views) {
    const want = local.focusedPane[v.id];
    if (want && leaves(v.layout).some((l) => l.pane === want)) focusedPane[v.id] = want;
  }
  viewsStore.set({ views, focusedPane });
  syncTabs(views);
}

/** Reloads on `views-changed` for this container and remembers the active tab; returns the unsubscribe. */
export function startViewsEvents(): () => void {
  const off = events.viewsChanged.listen((e) => {
    const { container } = viewsStore.get();
    if (container && sameContainer(container, e.payload.container)) void reload(container);
  });
  let active = shellStore.get().activeTab;
  const offShell = shellStore.subscribe(() => {
    if (shellStore.get().activeTab === active) return;
    active = shellStore.get().activeTab;
    persistLocal();
  });
  return () => {
    offShell();
    void off.then((fn) => fn());
  };
}

/** A new view (default name `View N`), made active with the picker open in its single empty pane. */
export async function createView(name?: string): Promise<ViewId | null> {
  const { container, views } = viewsStore.get();
  if (!container) return null;
  const r = await commands.viewsCreate(container, name ?? `View ${views.length + 1}`);
  if (r?.status !== "ok") return null;
  const view = r.data;
  viewsStore.set((s) => ({ ...s, views: [...s.views.filter((v) => v.id !== view.id), view] }));
  syncTabs(viewsStore.get().views);
  setActiveTab(view.id);
  const pane = leaves(view.layout)[0].pane;
  focusPane(view.id, pane);
  openPicker(view.id, pane);
  return view.id;
}

export async function renameView(id: ViewId, name: string): Promise<void> {
  const trimmed = name.trim();
  if (!trimmed) return;
  viewsStore.set((s) => ({ ...s, views: s.views.map((v) => (v.id === id ? { ...v, name: trimmed } : v)) }));
  await commands.viewsRename(id, trimmed);
}

export async function reorderViews(ids: ViewId[]): Promise<void> {
  const { container, views } = viewsStore.get();
  if (!container) return;
  const next = ids
    .map((id, order) => {
      const v = views.find((x) => x.id === id);
      return v ? { ...v, order } : null;
    })
    .filter((v): v is View => v !== null);
  viewsStore.set({ views: next });
  syncTabs(next);
  await commands.viewsReorder(container, ids);
}

/** Deletes the view; its sessions keep running (no `sessions_*` call). */
export async function deleteView(id: ViewId): Promise<void> {
  const views = viewsStore.get().views.filter((v) => v.id !== id);
  viewsStore.set((s) => ({ ...s, views, pickerFor: s.pickerFor?.viewId === id ? null : s.pickerFor }));
  syncTabs(views);
  await commands.viewsDelete(id);
}
