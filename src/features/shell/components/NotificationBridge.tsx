// Turns `task-changed` into one toast or OS notification per transition into needs-you, and keeps the Dock badge
// (owning window only) equal to the needs-you total over every open window's projects.
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect } from "react";
import { toast } from "@/components/ui";
import { commands, events, type AgentState, type Task, type TaskId, type WindowInfo } from "@/lib/bindings";
import { handbackLabel } from "@/lib/format";
import { projectsStore } from "@/lib/stores/projectsStore";
import { tasksStore } from "@/lib/stores/tasksStore";
import { activeView, viewTaskIds } from "@/features/views";
import { focusedWindow, trackWindowFocus } from "../focusTracker";
import { badgeTotal, enteredNeedsYou, ownsAppWide, planNotification } from "../notify";
import { openTaskInCurrentView } from "../openTask";
import { shellStore } from "../store";

let permission: Promise<boolean> | null = null;
function canNotify(): Promise<boolean> {
  permission ??= isPermissionGranted()
    .then((ok) => ok || requestPermission().then((p) => p === "granted"))
    .catch(() => false);
  return permission;
}

async function listWindows(): Promise<WindowInfo[]> {
  const w = await Promise.resolve(commands.windowsList()).catch(() => []);
  return Array.isArray(w) ? w : [];
}

async function isFocused(): Promise<boolean> {
  try {
    return await getCurrentWindow().isFocused();
  } catch {
    return document.hasFocus();
  }
}

async function onTask(task: Task, prev: Map<TaskId, AgentState>) {
  const entered = enteredNeedsYou(prev.get(task.id), task.state);
  prev.set(task.id, task.state);
  if (!entered) return;
  const me = shellStore.get().windowId;
  const windows = await listWindows();
  const ids = windows.map((w) => w.window_id);
  const view = activeView();
  const plan = planNotification({
    entered,
    windowFocused: projectsStore.get().order.includes(task.project_id) && (await isFocused()),
    taskVisibleInActiveView: Boolean(view && viewTaskIds(view).includes(task.id)),
    anyWindowFocused: ids.includes(focusedWindow() ?? ""),
    ownsAppWide: me !== null && ownsAppWide(me, ids),
  });
  const label = handbackLabel(task);
  if (plan.toast)
    toast(`${task.title} · ${label}`, { action: { label: "Open", onClick: () => openTaskInCurrentView(task.id) } });
  if (plan.os && (await canNotify())) sendNotification({ title: task.title, body: label });
}

async function updateBadge() {
  const me = shellStore.get().windowId;
  const windows = await listWindows();
  if (
    !me ||
    !ownsAppWide(
      me,
      windows.map((w) => w.window_id),
    )
  )
    return;
  const total = badgeTotal(tasksStore.get().needsYouByContainer, windows);
  try {
    await getCurrentWindow().setBadgeCount(total || undefined);
  } catch {
    // Outside Tauri there is no Dock.
  }
}

export function NotificationBridge(): null {
  const windowId = shellStore.useSelector((s) => s.windowId);
  useEffect(() => {
    if (!windowId) return;
    const prev = new Map<TaskId, AgentState>();
    for (const t of Object.values(tasksStore.get().byId)) prev.set(t.id, t.state);
    const offFocus = trackWindowFocus(windowId);
    const off = events.taskChanged.listen((e) => void onTask(e.payload.task, prev));
    let counts = tasksStore.get().needsYouByContainer;
    const offStore = tasksStore.subscribe(() => {
      if (tasksStore.get().needsYouByContainer === counts) return;
      counts = tasksStore.get().needsYouByContainer;
      void updateBadge();
    });
    void updateBadge();
    return () => {
      offFocus();
      offStore();
      void Promise.resolve(off).then((fn) => fn?.());
    };
  }, [windowId]);
  return null;
}
