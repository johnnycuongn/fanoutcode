// The views feature's public surface.
export * from "./layout";
export * from "./selectors";
export {
  viewsStore,
  activeView,
  viewById,
  focusedPaneOf,
  focusPane,
  splitFocusedPane,
  splitFocused,
  closeFocused,
  closePane,
  focusDirection,
  movePaneTo,
  setPaneRatio,
  setPaneContent,
  openPicker,
  closePicker,
  openTaskInView,
  openTeamPane,
  persistLayout,
} from "./store";
export type { ViewsState } from "./store";
export { loadViews, startViewsEvents, createView, renameView, reorderViews, deleteView } from "./viewsCrud";
export type * from "./types";
export { setPaneHandle, taskHandle, focusPaneContent } from "./paneHandles";
export type { PaneHandle } from "./paneHandles";
export { ViewCanvas } from "./components/ViewCanvas";
// Plan 04's TerminalPane lives in this feature (A5).
export { TerminalPane } from "./components/TerminalPane";
export type { TerminalPaneProps, TerminalPaneHandle } from "./components/TerminalPane";
export { TerminalPlaceholder } from "./components/TerminalPlaceholder";
export type { TerminalPlaceholderProps } from "./components/TerminalPlaceholder";
export { TaskPicker } from "./components/TaskPicker";
export { terminalStore, useTerminal, startListening as startTerminalEvents } from "./terminalStore";
export type { TerminalState, TerminalStatus } from "./terminalStore";
