import type { ReactElement } from "react";
import { formatCost, useBoardCost } from "../cost";

/** The board's total in the header's right corner (only rendered while the cost overview is on). */
export function BoardCost(): ReactElement | null {
  const board = useBoardCost();
  if (!board) return null;
  return (
    <span className="tabular-nums">
      <span className="text-ink">{formatCost(board)}</span> <span>on board</span>
    </span>
  );
}
