// ⌘⇧J by placement mode (§10.7 "Placement"), the one-line team summary the Swap strip shows, and the notice shown
// when the hook stream is missing what the canvas needs.
import type { TaskEvent, TeamPlacement } from "@/lib/bindings";
import { AGENT_TOOLS, asPreTool, asUnknown } from "./events";
import type { CanvasModel } from "./types";

export const DOCK_GROWN_FRACTION = 2 / 3;

export type TeamPlacementState = { grown: boolean; shown: "terminal" | "team"; paneOpen: boolean };
export type TeamPlacementAction =
  | { kind: "dock-fraction"; fraction: number | "restore" }
  | { kind: "show"; which: "terminal" | "team" }
  | { kind: "open-pane" }
  | { kind: "focus-pane" };

export function teamShortcut(
  placement: TeamPlacement,
  state: TeamPlacementState,
): { next: TeamPlacementState; action: TeamPlacementAction } {
  switch (placement) {
    case "DockGrows":
      return {
        next: { ...state, grown: !state.grown },
        action: { kind: "dock-fraction", fraction: state.grown ? "restore" : DOCK_GROWN_FRACTION },
      };
    case "Swap": {
      const which = state.shown === "terminal" ? "team" : "terminal";
      return { next: { ...state, shown: which }, action: { kind: "show", which } };
    }
    case "OwnPane":
      return state.paneOpen
        ? { next: state, action: { kind: "focus-pane" } }
        : { next: { ...state, paneOpen: true }, action: { kind: "open-pane" } };
  }
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** `2 agents working · step 2/6 active · 3 files`. */
export function teamSummary(model: CanvasModel): string {
  if (model.steps.length === 0 && model.files.length === 0) return "No team activity yet";
  const working = model.agents.filter((a) => a.working).length;
  const active = model.steps.filter((s) => s.status === "active").map((s) => s.number);
  const lowest = active.length > 0 ? String(Math.min(...active)) : "—";
  return [
    `${plural(working, "agent", "agents")} working`,
    `step ${lowest}/${model.steps.length} active`,
    plural(model.files.length, "file", "files"),
  ].join(" · ");
}

export function degradationNotice(events: TaskEvent[]): string | null {
  const unknown: string[] = [];
  let dispatches = false;
  let starts = false;
  for (const e of events) {
    const u = asUnknown(e);
    if (u && !unknown.includes(u.name)) unknown.push(u.name);
    const pre = asPreTool(e);
    if (pre && AGENT_TOOLS.includes(pre.tool)) dispatches = true;
    if (e.event.type === "SubagentStart") starts = true;
  }
  const parts: string[] = [];
  if (unknown.length > 0)
    parts.push(
      `Claude Code sent events this version does not understand: ${unknown.join(", ")}. The team view may be incomplete.`,
    );
  if (dispatches && !starts) parts.push("No SubagentStart events arrived; agents are drawn from Task dispatches only.");
  return parts.length > 0 ? parts.join(" ") : null;
}
