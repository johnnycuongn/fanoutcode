// One window's shell state: which window this is, its tabs (Board is tab zero), the sidebar and the workspace
// project focus. The sidebar's open state and section are remembered per container in localStorage.
import type { Container, ProjectId, TaskId, ViewId, WindowId, WindowInfo } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { containerKey } from "@/lib/stores/tasksStore";
import type { DockTab, ProjectLayer, SidebarSection, TabId } from "./types";

export type ShellState = {
  windowId: WindowId | null;
  container: Container | null;
  kind: "start" | "project" | "workspace";
  activeTab: TabId;
  tabOrder: TabId[];
  sidebarOpen: boolean;
  sidebarRemembered: boolean | null;
  sidebarSection: SidebarSection;
  windowWidth: number;
  focusedProject: ProjectId | null;
  projectLayer: ProjectLayer;
  pendingDockTab: { taskId: TaskId; tab: DockTab } | null;
};

/** Below this width the sidebar starts hidden, until the user opens or closes it (§10.1). */
export const SIDEBAR_MIN_WIDTH = 1100;

const initialWidth = () => (typeof window === "undefined" ? 1280 : window.innerWidth);

export const shellStore = createStore<ShellState>({
  windowId: null,
  container: null,
  kind: "start",
  activeTab: "board",
  tabOrder: ["board"],
  sidebarOpen: true,
  sidebarRemembered: null,
  sidebarSection: "tasks",
  windowWidth: initialWidth(),
  focusedProject: null,
  projectLayer: "MultiRoot",
  pendingDockTab: null,
});

const sidebarKey = (c: Container) => `tc:sidebar:${containerKey(c)}`;

function readSidebar(c: Container | null): { open: boolean | null; section: SidebarSection } {
  const fallback = { open: null, section: "tasks" as SidebarSection };
  if (!c) return fallback;
  try {
    const raw = localStorage.getItem(sidebarKey(c));
    const parsed = raw ? (JSON.parse(raw) as { open?: boolean | null; section?: SidebarSection }) : {};
    return { open: typeof parsed.open === "boolean" ? parsed.open : null, section: parsed.section ?? "tasks" };
  } catch {
    return fallback;
  }
}

function persistSidebar() {
  const { container, sidebarRemembered, sidebarSection } = shellStore.get();
  if (!container) return;
  try {
    localStorage.setItem(sidebarKey(container), JSON.stringify({ open: sidebarRemembered, section: sidebarSection }));
  } catch {
    // Storage blocked: the sidebar falls back to the width rule next time.
  }
}

export function resolveSidebarOpen(remembered: boolean | null, width: number): boolean {
  return remembered ?? width >= SIDEBAR_MIN_WIDTH;
}

export function initWindow(info: WindowInfo): void {
  const saved = readSidebar(info.container);
  const width = shellStore.get().windowWidth;
  shellStore.set({
    windowId: info.window_id,
    container: info.container,
    kind: info.container === null ? "start" : info.container.type === "Project" ? "project" : "workspace",
    activeTab: "board",
    tabOrder: ["board"],
    sidebarRemembered: saved.open,
    sidebarOpen: resolveSidebarOpen(saved.open, width),
    sidebarSection: saved.section,
    focusedProject: null,
    pendingDockTab: null,
  });
}

export function setTabOrder(ids: ViewId[]): void {
  const tabOrder: TabId[] = ["board", ...ids];
  const { activeTab } = shellStore.get();
  shellStore.set({ tabOrder, activeTab: tabOrder.includes(activeTab) ? activeTab : "board" });
}

export function setActiveTab(id: TabId): void {
  if (shellStore.get().tabOrder.includes(id)) shellStore.set({ activeTab: id });
}

export function cycleTab(delta: 1 | -1): void {
  const { tabOrder, activeTab } = shellStore.get();
  const i = Math.max(0, tabOrder.indexOf(activeTab));
  shellStore.set({ activeTab: tabOrder[(i + delta + tabOrder.length) % tabOrder.length] });
}

/** 1-based (⌘1 is the Board); an index beyond the tabs is ignored. */
export function selectTabIndex(n: number): void {
  const { tabOrder } = shellStore.get();
  if (n >= 1 && n <= tabOrder.length) shellStore.set({ activeTab: tabOrder[n - 1] });
}

export function toggleSidebar(): void {
  const open = !shellStore.get().sidebarOpen;
  shellStore.set({ sidebarOpen: open, sidebarRemembered: open });
  persistSidebar();
}

export function setSidebarSection(section: SidebarSection): void {
  shellStore.set({ sidebarSection: section });
  persistSidebar();
}

export function setWindowWidth(px: number): void {
  const { sidebarRemembered, windowWidth } = shellStore.get();
  if (px === windowWidth && shellStore.get().sidebarOpen === resolveSidebarOpen(sidebarRemembered, px)) return;
  shellStore.set({ windowWidth: px, sidebarOpen: resolveSidebarOpen(sidebarRemembered, px) });
}

export function setProjectLayer(layer: ProjectLayer): void {
  if (shellStore.get().projectLayer !== layer) shellStore.set({ projectLayer: layer });
}

/** ⌘N in a workspace: focus the Nth project, or clear the focus when it is already focused; out of range is ignored. */
export function toggleProjectFocus(index: number, projectIds: ProjectId[]): void {
  const id = projectIds[index - 1];
  if (!id) return;
  shellStore.set({ focusedProject: shellStore.get().focusedProject === id ? null : id });
}

export function setFocusedProject(id: ProjectId | null): void {
  shellStore.set({ focusedProject: id });
}

export function requestDockTab(taskId: TaskId, tab: DockTab): void {
  shellStore.set({ pendingDockTab: { taskId, tab } });
}

/** The dock tab asked for when `taskId` was opened, once (Plan 08 calls this when a task page mounts). */
export function consumeDockTab(taskId: TaskId): DockTab | null {
  const pending = shellStore.get().pendingDockTab;
  if (!pending || pending.taskId !== taskId) return null;
  shellStore.set({ pendingDockTab: null });
  return pending.tab;
}

/** What the Board shows: the focused project in the Rail layer, else every project (A5). */
export function boardProject(state: ShellState, projectIds: ProjectId[]): ProjectId | "all" {
  const { projectLayer, focusedProject } = state;
  return projectLayer === "Rail" && focusedProject && projectIds.includes(focusedProject) ? focusedProject : "all";
}
