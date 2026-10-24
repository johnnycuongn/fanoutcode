import { createContext, useContext } from "react";
import type { WindowInfo } from "@/lib/bindings";
import type { ConfirmOptions, OpenTarget } from "../types";

/** What every card needs from the board root, held in one stable value so memoised cards stay still. */
export type BoardCtx = {
  windows: WindowInfo[];
  currentWindowId: string;
  onOpenTask(taskId: string, target: OpenTarget): void;
  confirm(opts: ConfirmOptions): Promise<boolean>;
  openJira(taskId: string): void;
};

export const BoardContext = createContext<BoardCtx | null>(null);

export function useBoardCtx(): BoardCtx {
  const ctx = useContext(BoardContext);
  if (!ctx) throw new Error("A board component rendered outside <Board>");
  return ctx;
}
