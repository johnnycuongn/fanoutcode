// What the board does to tasks: the optimistic move, archive, rename, update and stop. Each call goes through `ipc`,
// writes the answer into the shared task cache, and turns a failure into an error toast (never a stuck card).
import type { ActionOutcome, Stage, Task } from "@/lib/bindings";
import { toast } from "@/components/ui";
import { GLOW_MS } from "@/lib/motion";
import { ipc } from "./ipc";
import { boardStore, laneOf, upsertTask } from "./store";
import { taskMap } from "./taskCache";
import type { ConfirmOptions, TaskPatchInput } from "./types";

const message = (err: unknown): string => (err instanceof Error ? err.message : String(err));

export const isWorking = (task: Pick<Task, "state">): boolean => task.state.type === "Working";

export function archiveConfirm(task: Pick<Task, "title">): ConfirmOptions {
  return {
    title: `Archive "${task.title}"?`,
    body: "The agent is still working. Archiving stops it; the memory folder is kept.",
    confirmLabel: "Archive",
    danger: true,
  };
}

export function stopConfirm(task: Pick<Task, "title">): ConfirmOptions {
  return { title: `Stop the agent on "${task.title}"?`, confirmLabel: "Stop agent", danger: true };
}

let glowTimer: ReturnType<typeof setTimeout> | undefined;

function clearDrop(): void {
  clearTimeout(glowTimer);
  boardStore.set({ lastDrop: null });
}

/** Marks where a card landed so its column glows for `GLOW_MS`. */
export function markDrop(taskId: string, stage: Stage, lane: string): void {
  clearTimeout(glowTimer);
  boardStore.set({ lastDrop: { taskId, stage, lane, at: Date.now() } });
  glowTimer = setTimeout(() => boardStore.set({ lastDrop: null }), GLOW_MS);
}

/** Applies the core's answer: the task it returns (if the board holds it) and the toasts it asks for. */
function settle(outcome: ActionOutcome | undefined): void {
  if (!outcome) return;
  if (outcome.task && taskMap()[outcome.task.id]) upsertTask(outcome.task);
  for (const text of outcome.toasts ?? []) toast.info(text);
}

/** Moves a card at once and asks the core; if the core refuses, the card goes back and the error is shown. */
export async function moveTask(taskId: string, to: Stage): Promise<void> {
  const task = taskMap()[taskId];
  if (!task || task.stage === to) return;
  const from = task.stage;
  upsertTask({ ...task, stage: to });
  markDrop(taskId, to, laneOf(boardStore.get(), task));
  try {
    settle(await ipc.tasksMove(taskId, to));
  } catch (err) {
    // Put back only the stage, so a change that arrived while the call was out is kept.
    const current = taskMap()[taskId];
    if (current && current.stage === to) upsertTask({ ...current, stage: from });
    if (boardStore.get().lastDrop?.taskId === taskId) clearDrop();
    toast.error(message(err));
  }
}

async function attempt(run: () => Promise<ActionOutcome | Task | void>): Promise<void> {
  try {
    const result = await run();
    if (result && "task" in result) settle(result);
    else if (result) upsertTask(result);
  } catch (err) {
    toast.error(message(err));
  }
}

export async function renameTask(taskId: string, title: string): Promise<void> {
  const next = title.trim();
  const task = taskMap()[taskId];
  if (!next || !task || next === task.title) return;
  await attempt(() => ipc.tasksRename(taskId, next));
}

export const updateTask = (taskId: string, patch: TaskPatchInput): Promise<void> =>
  attempt(() => ipc.tasksUpdate(taskId, patch));

export async function archiveTask(taskId: string, confirm: () => Promise<boolean>): Promise<void> {
  const task = taskMap()[taskId];
  if (!task) return;
  if (isWorking(task) && !(await confirm())) return;
  await attempt(() => ipc.tasksArchive(taskId));
}

export const restoreTask = (taskId: string): Promise<void> => attempt(() => ipc.tasksRestore(taskId));

export async function stopAgent(taskId: string, confirm: () => Promise<boolean>): Promise<void> {
  const session = taskMap()[taskId]?.primary_session;
  if (!session || !(await confirm())) return;
  await attempt(() => ipc.sessionsStop(session));
}
