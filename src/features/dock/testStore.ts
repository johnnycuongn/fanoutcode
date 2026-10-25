// The Test tab's runs per task. Plan 05's `tests_run` is start-or-attach with replay (A15): it starts the test command
// when nothing runs and otherwise attaches and replays the output so far, so a second window sees the whole run.
// `RunTests` is only the pipeline's bookkeeping; this store owns the output channel (Plan 04: Effect::RunTest is a
// no-op).
import { Channel } from "@tauri-apps/api/core";
import type { ProjectId, TaskId } from "@/lib/bindings";
import { errorMessage, live } from "@/lib/ipcResult";
import { createStore } from "@/lib/stores/createStore";

export const MAX_LINES = 5000;

export interface TestRun {
  lines: string[];
  partial: string;
  code: number | null;
  running: boolean;
  error?: string | null;
}

export type TestIpc = ReturnType<typeof live<"testsRun" | "testsAddLinked" | "tasksAction">>;

/** Channel frames arrive as JSON number arrays, or raw bytes. */
function toBytes(msg: unknown): Uint8Array {
  // isView, not instanceof: typed arrays from another realm (a test's TextEncoder) fail instanceof
  if (ArrayBuffer.isView(msg)) return new Uint8Array(msg.buffer, msg.byteOffset, msg.byteLength);
  if (msg instanceof ArrayBuffer || Object.prototype.toString.call(msg) === "[object ArrayBuffer]")
    return new Uint8Array(msg as ArrayBuffer);
  return Uint8Array.from(Array.isArray(msg) ? (msg as number[]) : []);
}

export function appendOutput(run: TestRun, chunk: Uint8Array, decoder: TextDecoder): TestRun {
  const text = run.partial + decoder.decode(chunk, { stream: true });
  const parts = text.split("\n");
  const partial = parts.pop() ?? "";
  if (parts.length === 0) return { ...run, partial };
  const lines = [...run.lines, ...parts.map((l) => l.replace(/\r$/, ""))];
  return { ...run, lines: lines.length > MAX_LINES ? lines.slice(lines.length - MAX_LINES) : lines, partial };
}

const started = (): TestRun => ({ lines: [], partial: "", code: null, running: true, error: null });

export function createTestStore(ipc: TestIpc, makeChannel: () => Channel<number[]> = () => new Channel<number[]>()) {
  const store = createStore<Record<string, TestRun>>({});
  const decoders = new Map<TaskId, TextDecoder>();
  const patch = (id: TaskId, fn: (r: TestRun) => TestRun) => store.set((s) => ({ ...s, [id]: fn(s[id] ?? started()) }));

  async function attach(taskId: TaskId): Promise<void> {
    const decoder = new TextDecoder();
    decoders.set(taskId, decoder);
    const channel = makeChannel();
    channel.onmessage = (msg) => {
      if (decoders.get(taskId) === decoder) patch(taskId, (r) => appendOutput(r, toBytes(msg), decoder));
    };
    try {
      const code = await ipc.testsRun(taskId, channel);
      if (decoders.get(taskId) === decoder) patch(taskId, (r) => ({ ...r, code, running: false }));
    } catch (e) {
      if (decoders.get(taskId) === decoder) patch(taskId, (r) => ({ ...r, running: false, error: errorMessage(e) }));
    }
  }

  return {
    store,
    attach(taskId: TaskId): Promise<void> {
      store.set((s) => ({ ...s, [taskId]: started() }));
      return attach(taskId);
    },
    async run(taskId: TaskId): Promise<void> {
      store.set((s) => ({ ...s, [taskId]: started() }));
      const attached = attach(taskId);
      try {
        await ipc.tasksAction(taskId, { type: "RunTests" });
      } catch {
        // the pipeline may refuse the bookkeeping (e.g. outside Test); the run itself goes on
      }
      await attached;
    },
    async fix(taskId: TaskId): Promise<void> {
      await ipc.tasksAction(taskId, { type: "FixTests" });
    },
    async addLinked(taskId: TaskId, projectId: ProjectId): Promise<void> {
      await ipc.testsAddLinked(taskId, projectId);
    },
    onTestFinished(taskId: TaskId, code: number): void {
      patch(taskId, (r) => ({ ...r, code, running: false }));
    },
  };
}

export type TestStore = ReturnType<typeof createTestStore>;

export const testStore = createTestStore(live("testsRun", "testsAddLinked", "tasksAction"));
