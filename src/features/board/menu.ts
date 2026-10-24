// The card's menu as data: one list of items that the context menu and the `…` menu both render. Selections call the
// board actions; nothing here touches the DOM.
import type { Priority, Task, WindowInfo } from "@/lib/bindings";
import { SWATCHES } from "@/lib/theme";
import { archiveConfirm, archiveTask, restoreTask, stopAgent, stopConfirm, updateTask } from "./actions";
import { setRenaming } from "./store";
import type { ConfirmOptions, OpenTarget } from "./types";

export const WORKTREE_LOCKED_HINT = "Set when the session started · use Start fresh session to change it";
export const NO_OTHER_WINDOW_HINT = "No other window shows this project";

export type MenuItemSpec = {
  id: string;
  label: string;
  /** Display text such as "⌘E", shown on the right unless a `hint` replaces it. */
  shortcut?: string;
  disabled?: boolean;
  hint?: string;
  checked?: boolean;
  danger?: boolean;
  /** A colour swatch name, drawn as a square before the label. */
  swatch?: string;
  children?: MenuItemSpec[];
  onSelect?: () => void;
};

export type MenuCtx = {
  windows: WindowInfo[];
  currentWindowId: string;
  onOpen(target: OpenTarget): void;
  confirm(opts: ConfirmOptions): Promise<boolean>;
  openJira(taskId: string): void;
};

const PRIORITIES: Priority[] = ["None", "Low", "Medium", "High"];
const capitalise = (word: string): string => word.charAt(0).toUpperCase() + word.slice(1);

function windowItems(task: Task, ctx: MenuCtx): MenuItemSpec[] {
  return ctx.windows
    .filter((w) => w.window_id !== ctx.currentWindowId && w.project_ids.includes(task.project_id))
    .map((w) => ({
      id: `window:${w.window_id}`,
      label: w.title,
      onSelect: () => ctx.onOpen({ kind: "window", window_id: w.window_id }),
    }));
}

function colourItems(task: Task): MenuItemSpec[] {
  const none: MenuItemSpec = {
    id: "color:none",
    label: "None",
    checked: task.color === null,
    onSelect: () => void updateTask(task.id, { color: null }),
  };
  const swatches = SWATCHES.map((swatch) => ({
    id: `color:${swatch}`,
    label: capitalise(swatch),
    swatch,
    checked: task.color === swatch,
    onSelect: () => void updateTask(task.id, { color: swatch }),
  }));
  return [none, ...swatches];
}

export function cardMenuItems(task: Task, ctx: MenuCtx): MenuItemSpec[] {
  const windows = windowItems(task, ctx);
  const started = task.primary_session !== null;
  return [
    { id: "open", label: "Open", onSelect: () => ctx.onOpen({ kind: "current" }) },
    {
      id: "open_new_view",
      label: "Open in new view",
      shortcut: "⌘ click",
      onSelect: () => ctx.onOpen({ kind: "new_view" }),
    },
    {
      id: "open_window",
      label: "Open in window",
      children: windows,
      disabled: windows.length === 0,
      hint: windows.length === 0 ? NO_OTHER_WINDOW_HINT : undefined,
    },
    { id: "rename", label: "Rename", shortcut: "⌘E", onSelect: () => setRenaming(task.id) },
    {
      id: "priority",
      label: "Priority",
      children: PRIORITIES.map((p) => ({
        id: `priority:${p}`,
        label: p,
        checked: task.priority === p,
        onSelect: () => void updateTask(task.id, { priority: p }),
      })),
    },
    { id: "color", label: "Colour", children: colourItems(task) },
    {
      id: "flag",
      label: task.flagged ? "Unflag" : "Flag",
      onSelect: () => void updateTask(task.id, { flagged: !task.flagged }),
    },
    {
      id: "hide",
      label: task.hidden ? "Unhide" : "Hide",
      onSelect: () => void updateTask(task.id, { hidden: !task.hidden }),
    },
    {
      id: "archive",
      label: task.archived ? "Restore" : "Archive",
      shortcut: "⌘⇧A",
      onSelect: () =>
        void (task.archived ? restoreTask(task.id) : archiveTask(task.id, () => ctx.confirm(archiveConfirm(task)))),
    },
    {
      id: "link_jira",
      label: task.jira_key ? "Change Jira issue…" : "Link Jira issue…",
      onSelect: () => ctx.openJira(task.id),
    },
    {
      id: "worktree",
      label: "Use worktree",
      checked: task.worktree === "On",
      disabled: started,
      hint: started ? WORKTREE_LOCKED_HINT : undefined,
      onSelect: () => void updateTask(task.id, { worktree: task.worktree === "On" ? "Off" : "On" }),
    },
    {
      id: "stop_agent",
      label: "Stop agent",
      danger: true,
      disabled: !started,
      onSelect: () => void stopAgent(task.id, () => ctx.confirm(stopConfirm(task))),
    },
  ];
}
