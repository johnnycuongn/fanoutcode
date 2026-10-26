import { afterEach, expect, test, vi, type Mock } from "vitest";
import type { Channel } from "@tauri-apps/api/core";
import type { LiveSpecEvent } from "@/lib/bindings";
import { makeTask } from "@/test/taskPageFixtures";
import { createSpecStore } from "./store";
import type { SpecIpc } from "./types";

function fakeIpc(over: Partial<SpecIpc> = {}): SpecIpc {
  return {
    memoryRead: vi.fn(async () => "# Spec"),
    memoryWrite: vi.fn(async () => {}),
    settingsGet: vi.fn(async () => ({ livespec_level: "Simple" })),
    settingsSet: vi.fn(async () => {}),
    tasksAction: vi.fn(async () => {}),
    livespecAnalyze: vi.fn(async () => {}),
    livespecCancel: vi.fn(async () => {}),
    sessionsOpen: vi.fn(async () => "sess-9"),
    ...over,
  } as unknown as SpecIpc;
}
const task = makeTask({ stage: "Spec", primary_session: null });
afterEach(() => vi.useRealTimers());

test("open reads spec.md and the project's livespec_level", async () => {
  const ipc = fakeIpc();
  const s = createSpecStore(ipc);
  await s.open(task);
  expect(ipc.memoryRead).toHaveBeenCalledWith("task-1", "spec.md");
  expect(s.store.get().byTask["task-1"]).toMatchObject({ text: "# Spec", loaded: true, error: null });
  expect(s.store.get().levelByProject["proj-1"]).toBe("Simple");
  expect(s.session("task-1")?.getState().level).toBe("Simple");
});
test("edits save through memory_write 500 ms after the last keystroke, once", async () => {
  vi.useFakeTimers();
  const ipc = fakeIpc();
  const s = createSpecStore(ipc);
  await s.open(task);
  s.setText("task-1", "a");
  s.setText("task-1", "ab");
  s.setText("task-1", "abc");
  vi.advanceTimersByTime(499);
  expect(ipc.memoryWrite).not.toHaveBeenCalled();
  vi.advanceTimersByTime(1);
  expect(ipc.memoryWrite).toHaveBeenCalledTimes(1);
  expect(ipc.memoryWrite).toHaveBeenCalledWith("task-1", "spec.md", "abc");
});
test("setLevel writes the project-scoped setting and moves the session", async () => {
  const ipc = fakeIpc();
  const s = createSpecStore(ipc);
  await s.open(task);
  await s.setLevel(task, "Heavy");
  expect(ipc.settingsSet).toHaveBeenCalledWith(
    expect.objectContaining({ project: "proj-1" }),
    "livespec_level",
    "Heavy",
  );
  expect(s.session("task-1")?.getState().level).toBe("Heavy");
});
test("approve flushes the save and then sends Approve", async () => {
  vi.useFakeTimers();
  const ipc = fakeIpc();
  const s = createSpecStore(ipc);
  await s.open(task);
  s.setText("task-1", "final");
  await expect(s.approve(task)).resolves.toBe(true);
  expect(ipc.memoryWrite).toHaveBeenCalledWith("task-1", "spec.md", "final");
  expect(ipc.tasksAction).toHaveBeenCalledWith("task-1", { type: "Approve" });
});
test("analysis saves spec.md before livespec_analyze reads it", async () => {
  vi.useFakeTimers();
  const ipc = fakeIpc();
  const s = createSpecStore(ipc, {
    makeChannel: () => ({ onmessage: () => {} }) as unknown as Channel<LiveSpecEvent>,
  });
  await s.open(task);
  const draft = "A spec long enough to be analysed by the notes model.";
  s.setText("task-1", draft);
  await vi.advanceTimersByTimeAsync(1200);
  expect(ipc.livespecAnalyze).toHaveBeenCalledWith("task-1", expect.anything());
  expect(ipc.memoryWrite).toHaveBeenLastCalledWith("task-1", "spec.md", draft);
  expect((ipc.memoryWrite as Mock).mock.invocationCallOrder.at(-1)!).toBeLessThan(
    (ipc.livespecAnalyze as Mock).mock.invocationCallOrder[0],
  );
});
test("a failed save blocks the advance and marks saving failed", async () => {
  const ipc = fakeIpc({
    memoryWrite: vi.fn(async () => {
      throw "disk full";
    }),
  } as Partial<SpecIpc>);
  const s = createSpecStore(ipc);
  await s.open(task);
  s.setText("task-1", "x");
  await expect(s.approve(task)).resolves.toBe(false);
  expect(ipc.tasksAction).not.toHaveBeenCalled();
  expect(s.store.get().byTask["task-1"].saving).toBe("failed");
});
test("a missing memory folder shows the error and keeps editing off", async () => {
  vi.useFakeTimers();
  const ipc = fakeIpc({
    memoryRead: vi.fn(async () => {
      throw "Memory folder missing: /x/2026-10-24-a";
    }),
  } as Partial<SpecIpc>);
  const s = createSpecStore(ipc);
  await s.open(task);
  expect(s.store.get().byTask["task-1"]).toMatchObject({
    loaded: false,
    error: "Memory folder missing: /x/2026-10-24-a",
  });
  s.setText("task-1", "typed");
  vi.advanceTimersByTime(1000);
  expect(ipc.memoryWrite).not.toHaveBeenCalled();
});
test("a folder memory_missing reports is an error, not an empty spec", async () => {
  const ipc = fakeIpc({
    memoryRead: vi.fn(async () => null),
    memoryMissing: vi.fn(async () => ["task-1"]),
  } as Partial<SpecIpc>);
  const s = createSpecStore(ipc);
  await s.open(task);
  expect(ipc.memoryMissing).toHaveBeenCalledWith({ type: "Project", id: "proj-1" });
  expect(s.store.get().byTask["task-1"]).toMatchObject({
    loaded: false,
    error: "Task memory folder is missing: /m/2026-10-24-a",
  });
});
test("a spec.md not written yet starts from the task's description", async () => {
  const ipc = fakeIpc({
    memoryRead: vi.fn(async (_id: string, file: string) => (file === "intent.md" ? "Fix the streak count" : null)),
    memoryMissing: vi.fn(async () => []),
  } as Partial<SpecIpc>);
  const s = createSpecStore(ipc);
  await s.open(task);
  expect(s.store.get().byTask["task-1"]).toMatchObject({ loaded: true, text: "Fix the streak count" });
});
test("toggleTerminal flips the strip and the ratio defaults to 0.4", async () => {
  const s = createSpecStore(fakeIpc());
  await s.open(task);
  expect(s.store.get().byTask["task-1"]).toMatchObject({ terminalExpanded: false, terminalRatio: 0.4 });
  s.toggleTerminal("task-1");
  expect(s.store.get().byTask["task-1"].terminalExpanded).toBe(true);
});
