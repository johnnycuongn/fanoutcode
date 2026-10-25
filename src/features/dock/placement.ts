// ⌘⇧J per Team view placement (§10.7): DockGrows opens the dock on Team and toggles its grown height; Swap trades
// the body between the terminal and the team; OwnPane asks the shell to open the team in its own pane.
import type { TeamPlacement } from "@/lib/bindings";
import type { DockTab } from "./types";

export type BodyMode = "terminal" | "team";
export interface GrowState {
  open: boolean;
  grown: boolean;
  body: BodyMode;
}

export function nextOnGrowShortcut(
  placement: TeamPlacement,
  s: GrowState,
): GrowState & { tab: DockTab | null; openTeamPane: boolean } {
  switch (placement) {
    case "DockGrows":
      return { ...s, open: true, grown: !(s.open && s.grown), tab: "team", openTeamPane: false };
    case "Swap":
      return { ...s, body: s.body === "terminal" ? "team" : "terminal", tab: null, openTeamPane: false };
    case "OwnPane":
      return { ...s, tab: null, openTeamPane: true };
  }
}
