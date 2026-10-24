// The imperative handles of the panes on screen: a task page's TaskPageHandle or a terminal's focus(). The view
// routes the task chords to the focused pane's handle; the palette's Reply focuses the task's terminal through it.
import type { PaneId } from "@/lib/bindings";
import type { TaskPageHandle } from "@/features/task-page";

export type PaneHandle = { kind: "task"; handle: TaskPageHandle } | { kind: "terminal"; handle: { focus(): void } };

const handles = new Map<PaneId, PaneHandle>();

export function setPaneHandle(pane: PaneId, h: PaneHandle | null): void {
  if (h) handles.set(pane, h);
  else handles.delete(pane);
}

export function taskHandle(pane: PaneId): TaskPageHandle | null {
  const h = handles.get(pane);
  return h?.kind === "task" ? h.handle : null;
}

/** Puts keyboard focus inside the pane: the task's terminal, or the terminal itself. */
export function focusPaneContent(pane: PaneId): void {
  const h = handles.get(pane);
  if (h?.kind === "task") h.handle.focusTerminal();
  else if (h?.kind === "terminal") h.handle.focus();
}
