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
export { fileRelationLabels, overlapsText, relationLabel } from "./labels";
export { OverlapTab } from "./components/OverlapTab";
export type { OverlapTabProps } from "./components/OverlapTab";
export type { OverlapState, OverlapCounts } from "./types";
