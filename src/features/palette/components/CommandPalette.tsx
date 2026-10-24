// ⌘K: go to a task, view, window or project, act on the focused task, and the window-wide commands. Mounted once by
// the shell; Enter runs the highlighted command and closes.
import { useEffect, useState, type ReactElement } from "react";
import { ComboboxList, Dialog, DialogContent, DialogTitle, KeyHint } from "@/components/ui";
import { commands, type WindowInfo } from "@/lib/bindings";
import { chordHint, registerHandler } from "@/lib/keyboard";
import { projectsStore, selectProjectList } from "@/lib/stores/projectsStore";
import { selectTaskList, tasksStore } from "@/lib/stores/tasksStore";
import { appScope, projectScope, useSetting } from "@/features/settings";
import { shellStore } from "@/features/shell";
import { focusedPaneOf, leaves, taskHandle, viewsStore } from "@/features/views";
import { buildCommands, type PaletteCommand } from "../commands";
import { closePalette, paletteStore, togglePalette } from "../store";

function useWindows(open: boolean): WindowInfo[] {
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  useEffect(() => {
    if (!open) return;
    let live = true;
    void Promise.resolve(commands.windowsList()).then((w) => live && setWindows(Array.isArray(w) ? w : []));
    return () => {
      live = false;
    };
  }, [open]);
  return windows;
}

export function CommandPalette(): ReactElement | null {
  const open = paletteStore.useSelector((s) => s.open);
  const shell = shellStore.use();
  const views = viewsStore.useSelector((s) => s.views);
  const focusedPanes = viewsStore.useSelector((s) => s.focusedPane);
  const tasks = tasksStore.useSelector(selectTaskList);
  const tasksById = tasksStore.useSelector((s) => s.byId);
  const projects = projectsStore.useSelector(selectProjectList);
  const windows = useWindows(open);
  const view = views.find((v) => v.id === shell.activeTab);
  const pane = view ? focusedPaneOf(view, focusedPanes) : null;
  const content = view && pane ? leaves(view.layout).find((l) => l.pane === pane)?.content : undefined;
  const focusedTask = content?.type === "Task" ? (tasksById[content.task_id] ?? null) : null;
  const mayCommit = useSetting(focusedTask ? projectScope(focusedTask.project_id) : appScope(), "may_commit") ?? false;
  useEffect(() => registerHandler("palette", togglePalette), []);
  if (!open || shell.kind === "start") return null;

  const list: PaletteCommand[] = buildCommands({
    kind: shell.kind,
    tasks,
    projects,
    views,
    windows,
    currentWindowId: shell.windowId ?? "",
    focusedTask,
    mayCommit,
    focusTaskTerminal: () => pane && taskHandle(pane)?.focusTerminal(),
  });
  const run = (id: string) => {
    const cmd = list.find((c) => c.id === id);
    closePalette();
    if (cmd) void cmd.run();
  };
  return (
    <Dialog open onOpenChange={(o) => !o && closePalette()}>
      <DialogContent size="lg" hideClose className="p-0">
        <DialogTitle className="sr-only">Command palette</DialogTitle>
        <ComboboxList
          label="Command palette"
          placeholder="Type a command or a task…"
          items={list.map((c) => ({ value: c.id, label: c.label, group: c.group }))}
          onSelect={run}
          renderItem={(item) => {
            const cmd = list.find((c) => c.id === item.value);
            return (
              <>
                <span className="min-w-0 truncate">{item.label}</span>
                <span className="ml-auto flex shrink-0 items-center gap-2 text-xs text-mute">
                  {cmd?.hint}
                  {cmd?.keys && <KeyHint keys={chordHint(cmd.keys)} size="sm" />}
                </span>
              </>
            );
          }}
        />
      </DialogContent>
    </Dialog>
  );
}
