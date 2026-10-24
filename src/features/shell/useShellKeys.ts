// The window-level chords: ⌘1–⌘9 (tabs in a project window, project focus in a workspace window), ⌘\, ⌘T, ⌘`, ⌘, and the
// one keyboard listener for the window. Scope comes from the focused element's `data-scope` ancestor; with nothing
// focused inside a pane, an open view counts as pane scope so ⌘D and ⌘W still work after switching tabs.
import { useEffect } from "react";
import { commands } from "@/lib/bindings";
import { openSettings } from "@/features/settings";
import { installKeyboard, registerHandler, type Scope } from "@/lib/keyboard";
import { selectTabIndex, shellStore, toggleProjectFocus, toggleSidebar } from "./store";

function activeScope(): { scope: Scope; terminalFocused: boolean } {
  const el = document.activeElement instanceof Element ? document.activeElement : null;
  const tagged = el?.closest("[data-scope]")?.getAttribute("data-scope");
  const fallback: Scope = shellStore.get().activeTab === "board" ? "global" : "pane";
  const scope: Scope = tagged === "pane" || tagged === "spec-page" ? tagged : fallback;
  return { scope, terminalFocused: Boolean(el?.closest(".xterm")) };
}

export function useShellKeys(opts: { newTask(): void; projectIds: string[] }): void {
  const { newTask, projectIds } = opts;
  const ids = projectIds.join(",");
  useEffect(() => installKeyboard(window, activeScope), []);
  useEffect(() => {
    const list = ids ? ids.split(",") : [];
    const offs = [
      registerHandler("tab.nth", ({ digit }) => {
        if (!digit) return;
        if (shellStore.get().kind === "workspace") toggleProjectFocus(digit, list);
        else selectTabIndex(digit);
      }),
      registerHandler("sidebar.toggle", () => toggleSidebar()),
      registerHandler("task.new", () => newTask()),
      registerHandler("settings", () => openSettings()),
      // If the native Window menu claims ⌘` first this never fires, which is fine.
      registerHandler("window.next", () => void commands.windowsFocusNext()),
    ];
    return () => offs.forEach((off) => off());
  }, [ids, newTask]);
}
