// The overlap feature's public surface (Plan 11, §7).
export {
  overlapStore,
  refreshOverlaps,
  startOverlapEvents,
  decide,
  overlapCount,
  pendingRows,
  viewOverlapCount,
} from "./store";
export { OverlapTab } from "./components/OverlapTab";
export { relationLabel } from "./labels";
export type { OverlapTabProps } from "./components/OverlapTab";
export { IntegratePane, mergeNeedsYou } from "./components/IntegratePane";
export type { IntegratePaneProps } from "./components/IntegratePane";
export { BaseBranchDialog } from "./components/BaseBranchDialog";
export type { BaseBranchDialogProps } from "./components/BaseBranchDialog";
export type { OverlapState, OverlapCounts } from "./types";
