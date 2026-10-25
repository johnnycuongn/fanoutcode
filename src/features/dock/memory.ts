// The Memory tab's rules: which file a stage opens on, which files the developer may edit, and when an agent's
// write means the file on screen must be re-read (never over unsaved edits, Review Focus 5).
import type { Stage, TaskEvent } from "@/lib/bindings";
import type { Live } from "@/lib/ipcResult";
import { createSaver } from "@/livespec/saver";

export type MemoryFile = "intent.md" | "spec.md" | "plan.md" | "review.md" | "commits.md" | "index.md";

export const MEMORY_FILES: { file: MemoryFile; label: string }[] = [
  { file: "intent.md", label: "Intent" },
  { file: "spec.md", label: "Spec" },
  { file: "plan.md", label: "Plan" },
  { file: "review.md", label: "Review" },
  { file: "commits.md", label: "Commits" },
  { file: "index.md", label: "Index" },
];

const STAGE_FILE: Record<Stage, MemoryFile> = {
  Spec: "intent.md",
  Plan: "plan.md",
  Build: "plan.md",
  Review: "review.md",
  Test: "plan.md",
  Commit: "commits.md",
  Done: "index.md",
};

export function memoryFileFor(stage: Stage): MemoryFile {
  return STAGE_FILE[stage];
}

/** index.md is the app's own (§8). */
export const isEditable = (f: MemoryFile) => f !== "index.md";

export const WRITE_TOOLS = ["Edit", "Write", "MultiEdit", "NotebookEdit"];

function writtenPath(input: unknown): string | null {
  if (typeof input !== "object" || input === null) return null;
  const { file_path, notebook_path } = input as { file_path?: unknown; notebook_path?: unknown };
  const path = file_path ?? notebook_path;
  return typeof path === "string" ? path : null;
}

export function shouldReload(ev: TaskEvent, file: MemoryFile, dirty: boolean): boolean {
  if (dirty || ev.event.type !== "PostToolUse" || !WRITE_TOOLS.includes(ev.event.tool)) return false;
  const path = writtenPath(ev.event.input);
  return path !== null && (path === file || path.endsWith(`/${file}`));
}

export type MemoryIpc = Live<"memoryRead" | "memoryWrite"> & Partial<Live<"memoryMissing">>;

/** The saver's key: the task and the file, so a save never lands in another task's or file's place. */
export const memoryKey = (taskId: string, file: MemoryFile) => `${taskId}\u0000${file}`;

/** One Memory tab's autosave (500 ms). `onSaved` runs when the newest edit for a key is on disk. */
export function createMemorySaver(ipc: MemoryIpc, onSaved: () => void, onError: (e: unknown) => void) {
  const latest = new Map<string, string>();
  const saver = createSaver<string>(
    async (key, text) => {
      const [taskId, file] = key.split("\u0000");
      await ipc.memoryWrite(taskId, file, text);
      if (latest.get(key) === text) onSaved();
    },
    500,
    onError,
  );
  return {
    edit(key: string, text: string): void {
      latest.set(key, text);
      saver.schedule(key, text);
    },
    flush: (key: string) => saver.flush(key),
  };
}
