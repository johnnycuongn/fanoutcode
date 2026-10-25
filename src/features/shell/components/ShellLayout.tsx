// A project or workspace window: chrome, tabs, [rail] sidebar and the active tab, then the attention strip. Hidden
// views stay mounted so their terminals keep state and switching costs no repaint.
import { useCallback, useEffect, useMemo, useState, type ReactElement } from "react";
import { commands, type TaskId, type WindowInfo } from "@/lib/bindings";
import { projectsStore, selectProjectList } from "@/lib/stores/projectsStore";
import * as boardModule from "@/features/board";
import { Board, type OpenTarget } from "@/features/board";
import { CommandPalette } from "@/features/palette";
import { SettingsDialog, settingsStore } from "@/features/settings";
import { Sidebar } from "@/features/sidebar";
import { createView, openTaskInView, TaskPicker, ViewCanvas, viewsStore } from "@/features/views";
import { addDroppedPath, useDocRows, WorkspaceSettings } from "@/features/workspace";
import { boardSettings } from "../boardSettings";
import { openTaskInCurrentView } from "../openTask";
import { boardProject, shellStore } from "../store";
import { useShellEvents } from "../useShellEvents";
import { useShellKeys } from "../useShellKeys";
import { AttentionStrip } from "./AttentionStrip";
import { NotificationBridge } from "./NotificationBridge";
import { ProjectRail } from "./ProjectRail";
import { BOARD_PANEL_ID, TabStrip } from "./TabStrip";
import { WindowChrome } from "./WindowChrome";

/** Plan 07's new-task sheet, when its module exports an opener. */
export function newTask(): void {
  (boardModule as { openNewTask?: () => void }).openNewTask?.();
}

function openFromBoard(taskId: TaskId, target: OpenTarget) {
  if (target.kind === "current") openTaskInCurrentView(taskId);
  else if (target.kind === "new_view") void createView().then((id) => id && openTaskInView(id, taskId));
  else void commands.windowsFocus(target.window_id);
}

function useWindowInfo(given?: WindowInfo): WindowInfo | null {
  const [info, setInfo] = useState<WindowInfo | null>(given ?? null);
  useEffect(() => {
    if (given) return;
    let live = true;
    void commands.windowsCurrent().then((r) => live && r?.status === "ok" && setInfo(r.data));
    return () => {
      live = false;
    };
  }, [given]);
  return info;
}

export function ShellLayout({ info: given }: { info?: WindowInfo }): ReactElement | null {
  const info = useWindowInfo(given);
  const { ready, dirty, title } = useShellEvents(info);
  const shell = shellStore.use();
  const views = viewsStore.useSelector((s) => s.views);
  const projects = projectsStore.useSelector(selectProjectList);
  const byScope = settingsStore.useSelector((s) => s.byScope);
  const projectIds = useMemo(() => projects.map((p) => p.id), [projects]);
  const docs = useDocRows(shell.container);
  const container = shell.container;
  const onAddPath = useCallback((path: string) => container && addDroppedPath(container, path), [container]);
  const onNewTask = useCallback(() => newTask(), []);
  useShellKeys({ newTask: onNewTask, projectIds });
  const settings = useMemo(
    () => (shell.container ? boardSettings(shell.container, byScope, projectIds) : null),
    [shell.container, byScope, projectIds],
  );
  if (!info || !ready || !shell.container || !settings) return null;
  const rail = shell.kind === "workspace" && shell.projectLayer === "Rail";
  return (
    <div className="flex h-full flex-col bg-page text-ink">
      <WindowChrome title={title} dirty={dirty} />
      <TabStrip />
      <div className="flex min-h-0 flex-1">
        {rail && <ProjectRail />}
        <Sidebar docs={docs} onAddPath={onAddPath} />
        <main className="relative min-w-0 flex-1">
          <div id={BOARD_PANEL_ID} hidden={shell.activeTab !== "board"} className="h-full overflow-auto">
            <Board
              container={shell.container}
              projects={projects}
              settings={settings}
              focusedProject={boardProject(shell, projectIds)}
              currentWindowId={shell.windowId ?? info.window_id}
              onOpenTask={openFromBoard}
              onNewTask={onNewTask}
            />
          </div>
          {views.map((v) => (
            <div key={v.id} hidden={shell.activeTab !== v.id} className="h-full">
              <ViewCanvas viewId={v.id} />
            </div>
          ))}
        </main>
      </div>
      <AttentionStrip />
      <TaskPicker />
      <CommandPalette />
      <NotificationBridge />
      <SettingsDialog
        container={shell.container}
        projects={projects}
        workspaceTab={
          shell.container.type === "Workspace" ? <WorkspaceSettings workspaceId={shell.container.id} /> : undefined
        }
      />
    </div>
  );
}
