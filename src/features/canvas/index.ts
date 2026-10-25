// The canvas feature's public surface (A5, A20): the Team tab, its strip and pane, the task-page wiring, the ⌘⇧J
// placement helper and the store's open/close.
export { TeamTab } from "./components/TeamTab";
export { TeamStrip } from "./components/TeamStrip";
export { CanvasPane } from "./components/CanvasPane";
export { renderTeam, onOpenTeamPane, teamCanvasContent } from "./teamPage";
export { teamShortcut, teamSummary, DOCK_GROWN_FRACTION } from "./placement";
export type { TeamPlacementState, TeamPlacementAction } from "./placement";
export { openCanvas, closeCanvas } from "./store";
export type { CanvasNode, CanvasEdge } from "./types";
