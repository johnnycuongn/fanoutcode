// The view's chords, registered only while its view is the active tab so hidden views never answer: splits, close,
// focus moves, the picker, and the task chords routed to the focused pane's task page.
import { useEffect } from "react";
import { registerHandler, type ShortcutId } from "@/lib/keyboard";
import type { TaskPageHandle } from "@/features/task-page";
import type { ViewId } from "@/lib/bindings";
import { focusPaneContent, taskHandle } from "./paneHandles";
import { closeFocused, focusDirection, focusedPaneOf, openPicker, splitFocused, viewById } from "./store";
import type { Dir } from "./types";

const TASK_CHORDS: Array<[ShortcutId, keyof TaskPageHandle]> = [
  ["task.primary", "primaryAction"],
  ["task.rename", "rename"],
  ["task.archive", "archive"],
  ["dock.toggle", "toggleDock"],
  ["team.grow", "growTeam"],
  ["terminalStrip.toggle", "toggleTerminalStrip"],
];

const FOCUS_CHORDS: Array<[ShortcutId, Dir]> = [
  ["pane.focusLeft", "left"],
  ["pane.focusUp", "up"],
  ["pane.focusRight", "right"],
  ["pane.focusDown", "down"],
];

export function useViewKeys(viewId: ViewId, active: boolean): void {
  useEffect(() => {
    if (!active) return;
    const focused = () => {
      const view = viewById(viewId);
      return view ? focusedPaneOf(view) : null;
    };
    const offs = [
      registerHandler("pane.splitRight", () => splitFocused("Horizontal")),
      registerHandler("pane.splitDown", () => splitFocused("Vertical")),
      registerHandler("pane.close", () => closeFocused()),
      registerHandler("pane.pick", () => {
        const pane = focused();
        if (pane) openPicker(viewId, pane);
      }),
      ...FOCUS_CHORDS.map(([id, dir]) =>
        registerHandler(id, () => {
          const to = focusDirection(dir);
          if (to) focusPaneContent(to);
        }),
      ),
      ...TASK_CHORDS.map(([id, method]) =>
        registerHandler(id, () => {
          const pane = focused();
          if (pane) taskHandle(pane)?.[method]();
        }),
      ),
    ];
    return () => offs.forEach((off) => off());
  }, [viewId, active]);
}
