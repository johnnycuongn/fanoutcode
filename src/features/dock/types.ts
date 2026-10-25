// The dock's tabs and its per-window state. Whether a pane's dock is open lives in that pane (Task 19), so a
// background task's stage change never flips another pane's dock.
import type { DockPosition, Stage } from "@/lib/bindings";

/** The same union as the shell's `DockTab` (its `requestDockTab` names one of these). */
export type DockTab = "memory" | "team" | "changes" | "ticket" | "test" | "commit";

export interface WatcherEntry {
  at: string;
  reason: string;
}

export interface DockState {
  position: DockPosition;
  tabByStage: Partial<Record<Stage, DockTab>>;
  heightByStage: Partial<Record<Stage, number>>;
  watcherLog: Record<string, WatcherEntry[]>;
}

/** Default: localStorage key "teamcode.dock". */
export interface DockPersist {
  read(): Partial<DockState> | null;
  write(s: DockState): void;
}
