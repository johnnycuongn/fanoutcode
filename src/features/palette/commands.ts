// What ⌘K offers, built from a snapshot the component assembles from the stores (this module reads no store).
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { toast } from "@/components/ui";
import { commands, type Project, type StageAction, type Task, type View, type WindowId } from "@/lib/bindings";
import type { WindowInfo } from "@/lib/bindings";
import { KEYMAP } from "@/lib/keyboard";
import { workspacesAddProject, workspacesCreate } from "@/lib/pendingCommands";
import * as board from "@/features/board";
import { appScope, openSettings, setSetting, THEMES } from "@/features/settings";
import { openTaskInCurrentView, setActiveTab, toggleProjectFocus, toggleSidebar } from "@/features/shell";
import { createView } from "@/features/views";
import { stageActions } from "./stageActions";

export type PaletteGroup = "Go to" | "Task" | "New" | "Workspace" | "Theme" | "Settings";

export type PaletteCommand = {
  id: string;
  group: PaletteGroup;
  label: string;
  hint?: string;
  keys?: string;
  run: () => void | Promise<void>;
};

export type PaletteCtx = {
  kind: "project" | "workspace";
  tasks: Task[];
  projects: Project[];
  views: View[];
  windows: WindowInfo[];
  currentWindowId: WindowId;
  focusedTask: Task | null;
  mayCommit: boolean;
  focusTaskTerminal: () => void;
};

const key = (id: keyof typeof KEYMAP, i = 0) => KEYMAP[id].chords[i];

async function act(id: string, action: StageAction) {
  const r = await commands.tasksAction(id, action);
  if (r?.status === "error") toast.error(r.error);
  else for (const t of r?.data.toasts ?? []) toast(t);
}

async function archive(id: string) {
  const r = await commands.tasksArchive(id);
  if (r?.status === "error") toast.error(r.error);
}

async function addProject(ctx: PaletteCtx) {
  const picked = await openDialog({ directory: true, title: "Add a project to the workspace" });
  if (typeof picked !== "string") return;
  const current = ctx.windows.find((w) => w.window_id === ctx.currentWindowId)?.container;
  let workspaceId = current?.type === "Workspace" ? current.id : null;
  if (!workspaceId) {
    const home = ctx.projects[0];
    const created = await workspacesCreate(`${home?.name ?? "New"} workspace`, home ? [home.id] : []);
    if (created.status === "error") return void toast.error(created.error);
    workspaceId = created.data.id;
  }
  const r = await workspacesAddProject(workspaceId, picked);
  if (r.status === "error") toast.error(r.error);
}

function goTo(ctx: PaletteCtx): PaletteCommand[] {
  const byId = Object.fromEntries(ctx.projects.map((p) => [p.id, p]));
  return [
    ...ctx.tasks
      .filter((t) => !t.archived)
      .map((t) => ({
        id: `task:${t.id}`,
        group: "Go to" as const,
        label: `Go to task: ${t.title}`,
        hint: [ctx.kind === "workspace" ? byId[t.project_id]?.name : null, t.stage].filter(Boolean).join(" · "),
        run: () => openTaskInCurrentView(t.id),
      })),
    ...ctx.views.map((v) => ({
      id: `view:${v.id}`,
      group: "Go to" as const,
      label: `Go to view: ${v.name}`,
      run: () => setActiveTab(v.id),
    })),
    ...ctx.windows
      .filter((w) => w.window_id !== ctx.currentWindowId)
      .map((w) => ({
        id: `window:${w.window_id}`,
        group: "Go to" as const,
        label: `Go to window: ${w.title}`,
        run: () => void commands.windowsFocus(w.window_id),
      })),
    ...(ctx.kind === "workspace"
      ? ctx.projects.map((p, i) => ({
          id: `focus:${p.id}`,
          group: "Go to" as const,
          label: `Focus project: ${p.name}`,
          keys: i < 9 ? key("tab.nth", i) : undefined,
          run: () =>
            toggleProjectFocus(
              i + 1,
              ctx.projects.map((x) => x.id),
            ),
        }))
      : []),
  ];
}

function taskCommands(ctx: PaletteCtx): PaletteCommand[] {
  const task = ctx.focusedTask;
  if (!task) return [];
  const id = task.id;
  return [
    ...stageActions(task, ctx.mayCommit).map((s) => ({
      id: `stage:${s.action}`,
      group: "Task" as const,
      label: s.label,
      run: () => act(id, { type: s.action }),
    })),
    { id: "fresh", group: "Task", label: "Start fresh session", run: () => act(id, { type: "StartFresh" }) },
    { id: "reply", group: "Task", label: "Reply", run: () => ctx.focusTaskTerminal() },
    { id: "archive", group: "Task", label: "Archive task", keys: key("task.archive"), run: () => archive(id) },
  ];
}

export function buildCommands(ctx: PaletteCtx): PaletteCommand[] {
  return [
    ...goTo(ctx),
    {
      id: "new-task",
      group: "New",
      label: "New task",
      keys: key("task.new"),
      run: () => (board as { openNewTask?: () => void }).openNewTask?.(),
    },
    { id: "new-view", group: "New", label: "New view", keys: key("view.new"), run: () => void createView() },
    ...taskCommands(ctx),
    { id: "add-project", group: "Workspace", label: "Add project to workspace…", run: () => addProject(ctx) },
    ...THEMES.map((t) => ({
      id: `theme:${t}`,
      group: "Theme" as const,
      label: `Theme: ${t}`,
      run: () => setSetting(appScope(), "theme", t),
    })),
    { id: "settings", group: "Settings", label: "Open settings", keys: key("settings"), run: () => openSettings() },
    { id: "sidebar", group: "Settings", label: "Toggle sidebar", keys: key("sidebar.toggle"), run: toggleSidebar },
  ];
}
