// The Spec stage's state per task: spec.md's text with a 500 ms autosave, the project's LiveSpec level (remembered
// per project through `settings_set`), the SpecSession that reads along, and the terminal strip below the editor.
// "Approve spec" flushes the save first and refuses to advance when spec.md is not on disk (Review Focus 4).
import { Channel } from "@tauri-apps/api/core";
import type { LiveSpecEvent, LiveSpecLevel, Scope, Task, TaskId } from "@/lib/bindings";
import { live } from "@/lib/ipcResult";
import { createStore } from "@/lib/stores/createStore";
import { createSaver } from "@/livespec/saver";
import { errorMessage, SpecSession } from "./specSession";
import type { SpecIpc, SpecState, SpecTaskState } from "./types";

export const SAVE_DELAY_MS = 500;
export const DEFAULT_TERMINAL_RATIO = 0.4;
export const SAVE_FAILED = "Couldn't save the spec";

const SPEC = "spec.md";

const taskScope = (t: Task): Scope => ({ workspace: t.workspace_id, project: t.project_id, task: t.id });
const projectScope = (t: Task): Scope => ({ workspace: t.workspace_id, project: t.project_id, task: null });

const fresh = (): SpecTaskState => ({
  text: "",
  loaded: false,
  error: null,
  saving: "idle",
  terminalExpanded: false,
  terminalRatio: DEFAULT_TERMINAL_RATIO,
});

export function createSpecStore(
  ipc: SpecIpc,
  opts: { saveDelayMs?: number; debounceMs?: number; makeChannel?: () => Channel<LiveSpecEvent> } = {},
) {
  const store = createStore<SpecState>({ byTask: {}, levelByProject: {} });
  const sessions = new Map<TaskId, SpecSession>();
  const projectOf = new Map<TaskId, string>();
  const makeChannel = opts.makeChannel ?? (() => new Channel<LiveSpecEvent>());

  const patch = (taskId: TaskId, p: Partial<SpecTaskState>) =>
    store.set((s) => ({ ...s, byTask: { ...s.byTask, [taskId]: { ...(s.byTask[taskId] ?? fresh()), ...p } } }));

  const saver = createSaver<string>(
    async (taskId, text) => {
      try {
        await ipc.memoryWrite(taskId, SPEC, text);
        patch(taskId, { saving: "saved" });
      } catch (e) {
        patch(taskId, { saving: "failed" });
        throw e;
      }
    },
    opts.saveDelayMs ?? SAVE_DELAY_MS,
    () => {},
  );
  const flush = (taskId: TaskId) => saver.flush(taskId);

  async function readSpec(task: Task): Promise<string> {
    const text = await ipc.memoryRead(task.id, SPEC);
    if (text !== null) return text;
    // A missing folder reads as nothing; only memory_missing tells it from a spec.md not written yet (§8).
    const missing = (await ipc.memoryMissing?.({ type: "Project", id: task.project_id })) ?? [];
    if (missing.includes(task.id)) throw `Task memory folder is missing: ${task.memory_dir}`;
    return "";
  }

  async function open(task: Task): Promise<void> {
    if (sessions.has(task.id) || store.get().byTask[task.id]?.loaded) return;
    projectOf.set(task.id, task.project_id);
    patch(task.id, {});
    const [text, settings] = await Promise.allSettled([readSpec(task), ipc.settingsGet(taskScope(task))]);
    if (text.status === "rejected") {
      patch(task.id, { loaded: false, error: errorMessage(text.reason) });
      return;
    }
    const level: LiveSpecLevel = settings.status === "fulfilled" ? (settings.value.livespec_level ?? "Off") : "Off";
    store.set((s) => ({ ...s, levelByProject: { ...s.levelByProject, [task.project_id]: level } }));
    const session = new SpecSession(task.id, text.value, level, {
      analyze: async (id, onEvent) => {
        if (!(await flush(id))) throw SAVE_FAILED; // livespec_analyze reads spec.md from disk (A14)
        const channel = makeChannel();
        channel.onmessage = onEvent;
        await ipc.livespecAnalyze(id, channel);
      },
      cancel: async (id) => {
        await ipc.livespecCancel(id);
      },
      ...(opts.debounceMs !== undefined ? { debounceMs: () => opts.debounceMs! } : {}),
    });
    sessions.get(task.id)?.dispose();
    sessions.set(task.id, session);
    patch(task.id, { text: text.value, loaded: true, error: null });
  }

  async function close(taskId: TaskId): Promise<void> {
    await flush(taskId);
    sessions.get(taskId)?.dispose();
    sessions.delete(taskId);
    projectOf.delete(taskId);
    store.set((s) => {
      const byTask = { ...s.byTask };
      delete byTask[taskId];
      return { ...s, byTask };
    });
  }

  function setText(taskId: TaskId, text: string): void {
    const st = store.get().byTask[taskId];
    if (!st?.loaded) return; // editing is off until spec.md was read (memory missing, §8)
    patch(taskId, { text, saving: "idle" });
    saver.schedule(taskId, text);
    sessions.get(taskId)?.setDraft(text);
  }

  async function setLevel(task: Task, level: LiveSpecLevel): Promise<void> {
    await ipc.settingsSet(projectScope(task), "livespec_level", level);
    store.set((s) => ({ ...s, levelByProject: { ...s.levelByProject, [task.project_id]: level } }));
    for (const [id, session] of sessions) if (projectOf.get(id) === task.project_id) session.setLevel(level);
  }

  async function approve(task: Task): Promise<boolean> {
    if (!(await flush(task.id))) return false;
    await ipc.tasksAction(task.id, { type: "Approve" });
    return true;
  }

  return {
    store,
    open,
    close,
    setText,
    setLevel,
    approve,
    session: (taskId: TaskId): SpecSession | null => sessions.get(taskId) ?? null,
    startSession: (taskId: TaskId) => ipc.sessionsOpen(taskId),
    toggleTerminal(taskId: TaskId): void {
      patch(taskId, { terminalExpanded: !(store.get().byTask[taskId]?.terminalExpanded ?? false) });
    },
    setTerminalRatio(taskId: TaskId, ratio: number): void {
      patch(taskId, { terminalRatio: ratio });
    },
  };
}

export type SpecStore = ReturnType<typeof createSpecStore>;

export const specStore = createSpecStore(
  live(
    "memoryRead",
    "memoryWrite",
    "memoryMissing",
    "settingsGet",
    "settingsSet",
    "tasksAction",
    "livespecAnalyze",
    "livespecCancel",
    "sessionsOpen",
  ),
);
