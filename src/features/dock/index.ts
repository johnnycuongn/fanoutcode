// The dock feature's public surface.
export { Dock } from "./components/Dock";
export type { DockProps } from "./components/Dock";
export { DockSplit } from "./components/DockSplit";
export { dockStore, createDockStore } from "./store";
export type { DockStore } from "./store";
export type { DockTab, DockState, WatcherEntry } from "./types";
export { DOCK_TABS, visibleTabs, defaultTabFor, DEFAULT_HEIGHT, GROWN_HEIGHT } from "./tabRules";
export { nextOnGrowShortcut } from "./placement";
export type { BodyMode, GrowState } from "./placement";
export { MemoryTab } from "./components/MemoryTab";
export { TicketTab } from "./components/TicketTab";
export { TestTab } from "./components/TestTab";
export { CommitTab } from "./components/CommitTab";
