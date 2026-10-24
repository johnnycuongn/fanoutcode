import { useEffect, type RefObject } from "react";
import { archiveConfirm, archiveTask, restoreTask } from "./actions";
import { boardStore, setRenaming } from "./store";
import { taskMap } from "./taskCache";
import type { ConfirmOptions } from "./types";

/** ⌘E renames and ⌘⇧A archives (or restores) the focused card. Only handled keys are stopped, so the rest reach the window map. */
export function useBoardShortcuts(
  ref: RefObject<HTMLElement | null>,
  ctx: { confirm(opts: ConfirmOptions): Promise<boolean> },
): void {
  const { confirm } = ctx;
  useEffect(() => {
    const root = ref.current;
    if (!root) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (!e.metaKey || e.altKey || e.ctrlKey) return;
      const key = e.key.toLowerCase();
      const id = boardStore.get().focusedTaskId;
      const task = id ? taskMap()[id] : undefined;
      if (!task) return;
      if (key === "e" && !e.shiftKey) {
        setRenaming(task.id);
      } else if (key === "a" && e.shiftKey) {
        void (task.archived ? restoreTask(task.id) : archiveTask(task.id, () => confirm(archiveConfirm(task))));
      } else {
        return;
      }
      e.preventDefault();
      e.stopPropagation();
    };
    root.addEventListener("keydown", onKeyDown);
    return () => root.removeEventListener("keydown", onKeyDown);
  }, [ref, confirm]);
}
