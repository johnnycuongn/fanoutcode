// One window's data: the first load of settings, projects, tasks and views, the event subscriptions, the window
// width feeding the sidebar rule, the workspace dirty flag, and the native title.
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { events, type Container, type WindowInfo } from "@/lib/bindings";
import { loadProjects, projectsStore } from "@/lib/stores/projectsStore";
import { containerKey, loadTasks, setContainerProjects, startTasksEvents } from "@/lib/stores/tasksStore";
import { appScope, loadSettings, projectScope, scopeFor } from "@/features/settings";
import { loadViews, startViewsEvents } from "@/features/views";
import { initWindow, setProjectLayer, setWindowWidth } from "./store";

async function firstLoad(info: WindowInfo, container: Container) {
  setContainerProjects(info.project_ids);
  const [, resolved] = await Promise.all([
    loadSettings(appScope()),
    loadSettings(scopeFor(container)),
    loadProjects(info.project_ids),
    loadTasks(container),
    loadViews(container),
    ...(container.type === "Workspace" ? info.project_ids.map((id) => loadSettings(projectScope(id))) : []),
  ]);
  if (resolved?.workspace_project_layer) setProjectLayer(resolved.workspace_project_layer);
}

function setNativeTitle(title: string) {
  try {
    void getCurrentWindow()
      .setTitle(title)
      .catch(() => {});
  } catch {
    // Outside Tauri there is no native window.
  }
}

/** Initialises the window from `info` and loads it; `ready` turns true once the first load is in. */
export function useShellEvents(info: WindowInfo | null): { ready: boolean; dirty: boolean; title: string } {
  const [ready, setReady] = useState(false);
  const [dirty, setDirty] = useState(false);
  const container = info?.container ?? null;
  const key = container ? containerKey(container) : "";
  const name = projectsStore.useSelector((s) =>
    container?.type === "Project" ? s.byId[container.id]?.name : undefined,
  );
  const title = name ?? info?.title ?? "Fanout Code";

  useEffect(() => {
    if (!info || !info.container) return;
    const c = info.container;
    let live = true;
    initWindow(info);
    void firstLoad(info, c).finally(() => live && setReady(true));
    const offTasks = startTasksEvents();
    const offViews = startViewsEvents();
    const offWorkspace = events.workspaceChanged.listen((e) => {
      if (c.type === "Workspace" && e.payload.workspace.id === c.id) setDirty(e.payload.workspace.dirty);
    });
    const onResize = () => setWindowWidth(window.innerWidth);
    onResize();
    const observer = new ResizeObserver(onResize);
    observer.observe(document.body);
    return () => {
      live = false;
      offTasks();
      offViews();
      void Promise.resolve(offWorkspace).then((fn) => fn?.());
      observer.disconnect();
    };
    // The window's container never changes while it is mounted; `key` stands for it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  useEffect(() => setNativeTitle(dirty ? `${title} •` : title), [title, dirty]);
  return { ready, dirty, title };
}
