// Which dock tabs a stage shows, which one it opens on, and what entering a stage does to the dock (§6, §10.3).
import type { Stage } from "@/lib/bindings";
import type { DockTab } from "./types";

export const DOCK_TABS: { id: DockTab; label: string }[] = [
  { id: "memory", label: "Memory" },
  { id: "team", label: "Team" },
  { id: "changes", label: "Changes" },
  { id: "overlap", label: "Overlap" },
  { id: "ticket", label: "Ticket" },
  { id: "test", label: "Test" },
  { id: "commit", label: "Commit" },
];

export const DEFAULT_HEIGHT = 0.34;
export const GROWN_HEIGHT = 0.67;

/** Overlap shows in every stage while the task has an overlap (§7), so single-task projects never see it. */
export function visibleTabs(stage: Stage, hasTestCommand: boolean, hasOverlap = false): DockTab[] {
  const tabs: DockTab[] = ["memory", "team", "changes", ...(hasOverlap ? (["overlap"] as const) : []), "ticket"];
  if (hasTestCommand) tabs.push("test");
  if (stage === "Commit") tabs.push("commit");
  return tabs;
}

const DEFAULT_TAB: Record<Stage, DockTab> = {
  Spec: "memory",
  Plan: "memory",
  Build: "team",
  Review: "changes",
  Test: "test",
  Commit: "commit",
  Done: "memory",
};

export function defaultTabFor(stage: Stage): DockTab {
  return DEFAULT_TAB[stage];
}

export function resolveTab(stage: Stage, remembered: DockTab | undefined, visible: DockTab[]): DockTab {
  if (remembered && visible.includes(remembered)) return remembered;
  const fallback = defaultTabFor(stage);
  return visible.includes(fallback) ? fallback : (visible[0] ?? "memory");
}

/** `null` leaves the dock as it is. */
export function entryState(stage: Stage): { open: boolean | null; grown: boolean | null } {
  if (stage === "Done") return { open: null, grown: null };
  if (stage === "Spec") return { open: false, grown: false };
  return { open: true, grown: stage === "Build" };
}
