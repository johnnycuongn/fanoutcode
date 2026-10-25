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
export type { OverlapState, OverlapCounts } from "./types";
