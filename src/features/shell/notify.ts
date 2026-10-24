// The pure rules behind toasts, OS notifications and the Dock badge (Review Focus 4): one notice per transition
// into needs-you, a toast only in the focused window when the task is not on screen, an OS notification only when
// no window is focused, and exactly one window (the owner) for the app-wide effects.
import type { AgentState, WindowId, WindowInfo } from "@/lib/bindings";

export function enteredNeedsYou(prev: AgentState | undefined, next: AgentState): boolean {
  return next.type === "NeedsYou" && prev?.type !== "NeedsYou";
}

export function planNotification(a: {
  entered: boolean;
  windowFocused: boolean;
  taskVisibleInActiveView: boolean;
  anyWindowFocused: boolean;
  ownsAppWide: boolean;
}): { toast: boolean; os: boolean } {
  return {
    toast: a.entered && a.windowFocused && !a.taskVisibleInActiveView,
    os: a.entered && !a.anyWindowFocused && a.ownsAppWide,
  };
}

/** The lexicographically smallest window id owns the OS notifications and the Dock badge. */
export function ownsAppWide(windowId: WindowId, all: WindowId[]): boolean {
  return all.length === 0 || [...all].sort()[0] === windowId;
}

/** Needs-you tasks over the distinct projects of every open window. */
export function badgeTotal(counts: Record<string, number>, windows: WindowInfo[]): number {
  const projects = new Set(windows.flatMap((w) => w.project_ids));
  let total = 0;
  for (const id of projects) total += counts[`project:${id}`] ?? 0;
  return total;
}
