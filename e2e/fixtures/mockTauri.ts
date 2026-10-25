// The Tauri IPC replaced in-page for the smoke suite (src/main.tsx installs it in `e2e` mode before the first
// render). Handlers answer raw values by snake_case command name; tauri-specta's wrappers add the Result envelope.
// An app command with no handler is recorded in `window.__E2E_UNMOCKED__` (the spec fails naming it) and answered
// with null; plugin calls (window, notification, dialog) are answered with null silently.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Container, LayoutNode, View } from "../../src/lib/bindings";
import { COST, EMPTY_PLAN, EVENTS, PLANS, PROJECT, SETTINGS, TASKS, WINDOW } from "./seed";

type Args = Record<string, unknown>;

// Views the shell creates while the suite runs (opening a task makes one), kept in page memory.
const views: View[] = [];
function createView(args: Args): View {
  const view: View = {
    id: `v-${views.length + 1}`,
    container: args.container as Container,
    name: String(args.name),
    order: views.length,
    layout: { type: "Leaf", pane: `pane-${views.length + 1}`, content: { type: "Empty" } },
  };
  views.push(view);
  return view;
}
function updateView(args: Args, patch: Partial<View>): View | null {
  const i = views.findIndex((v) => v.id === args.id);
  if (i < 0) return null;
  views[i] = { ...views[i], ...patch };
  return views[i];
}

const taskOf = (args: Args) => TASKS.find((t) => t.id === (args.id ?? args.taskId)) ?? null;

export const HANDLERS: Record<string, (args: Args) => unknown> = {
  settings_first_run_pending: () => false,
  windows_current: () => WINDOW,
  windows_list: () => [WINDOW],
  windows_recent: () => [],
  projects_list: () => [PROJECT],
  settings_get: () => SETTINGS,
  views_list: () => views,
  views_create: createView,
  views_update_layout: (args) => updateView(args, { layout: args.layout as LayoutNode }),
  views_rename: (args) => updateView(args, { name: String(args.name) }),
  views_reorder: () => null,
  tasks_list: () => TASKS,
  tasks_get: (args) => taskOf(args),
  tasks_activity: () => [],
  memory_missing: () => [],
  memory_read: () => "",
  hooks_list: (args) => EVENTS[String(args.taskId)] ?? [],
  hooks_plan_steps: (args) => PLANS[String(args.taskId)] ?? EMPTY_PLAN,
  git_changes: () => [],
  cost_summary: () => COST,
  canvas_positions_get: () => [],
  canvas_positions_set: () => null,
  sessions_open: () => "s-0e1",
  sessions_attach: () => ({ lines: 0, dropped: 0, bytes: 0, cols: 80, rows: 24 }),
  sessions_resize: () => null,
  sessions_write: () => null,
  sessions_detach: () => null,
};

declare global {
  interface Window {
    __E2E_UNMOCKED__?: string[];
  }
}

function handle(cmd: string, payload?: unknown): unknown {
  const handler = HANDLERS[cmd];
  if (handler) return handler((payload ?? {}) as Args);
  if (!cmd.startsWith("plugin:")) (window.__E2E_UNMOCKED__ ??= []).push(cmd);
  return null;
}

export function installTauriMocks(): void {
  mockWindows("w-1");
  mockIPC(handle, { shouldMockEvents: true });
}
