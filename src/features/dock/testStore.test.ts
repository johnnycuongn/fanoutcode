import { expect, test, vi } from "vitest";
import type { Channel } from "@tauri-apps/api/core";
import { appendOutput, createTestStore, MAX_LINES, type TestIpc, type TestRun } from "./testStore";

const enc = new TextEncoder();
const empty: TestRun = { lines: [], partial: "", code: null, running: true };
const channel = () => ({ onmessage: () => {} }) as unknown as Channel<number[]>;
test("appendOutput splits lines and keeps a partial tail across chunks", () => {
  const d = new TextDecoder();
  let r = appendOutput(empty, enc.encode("PASS a\nFAI"), d);
  expect(r).toMatchObject({ lines: ["PASS a"], partial: "FAI" });
  r = appendOutput(r, enc.encode("L b\n"), d);
  expect(r).toMatchObject({ lines: ["PASS a", "FAIL b"], partial: "" });
});
test("appendOutput caps at MAX_LINES, dropping the oldest", () => {
  const r = appendOutput(
    empty,
    enc.encode(Array.from({ length: MAX_LINES + 10 }, (_, i) => `l${i}`).join("\n") + "\n"),
    new TextDecoder(),
  );
  expect(r.lines).toHaveLength(MAX_LINES);
  expect(r.lines[0]).toBe("l10");
});
function fakeIpc(code = 1): TestIpc & { emit: (s: string) => void } {
  const f = {
    emit: (() => {}) as (s: string) => void,
    testsRun: vi.fn((_t: string, ch: { onmessage: (b: Uint8Array) => void }) => {
      f.emit = (s) => ch.onmessage(enc.encode(s));
      return new Promise<number>((res) => setTimeout(() => res(code), 0));
    }),
    testsAddLinked: vi.fn(async () => {}),
    tasksAction: vi.fn(async () => {}),
  };
  return f as unknown as TestIpc & { emit: (s: string) => void };
}
test("run sends RunTests, attaches, streams output and records the exit code", async () => {
  vi.useFakeTimers();
  const ipc = fakeIpc(1);
  const s = createTestStore(ipc, channel);
  const p = s.run("task-1");
  expect(ipc.tasksAction).toHaveBeenCalledWith("task-1", { type: "RunTests" });
  ipc.emit("1 failing\n");
  expect(s.store.get()["task-1"]).toMatchObject({ lines: ["1 failing"], running: true });
  await vi.runAllTimersAsync();
  await p;
  expect(s.store.get()["task-1"]).toMatchObject({ code: 1, running: false });
  vi.useRealTimers();
});
test("a new run clears the previous output; test-finished from elsewhere sets the code", async () => {
  vi.useFakeTimers();
  const ipc = fakeIpc(0);
  const s = createTestStore(ipc, channel);
  s.onTestFinished("task-1", 1);
  expect(s.store.get()["task-1"].code).toBe(1);
  const p = s.run("task-1");
  expect(s.store.get()["task-1"]).toMatchObject({ lines: [], code: null, running: true });
  await vi.runAllTimersAsync();
  await p;
  vi.useRealTimers();
});
test("fix sends FixTests and addLinked calls tests_add_linked", async () => {
  const ipc = fakeIpc();
  const s = createTestStore(ipc);
  await s.fix("task-1");
  await s.addLinked("task-1", "proj-2");
  expect(ipc.tasksAction).toHaveBeenCalledWith("task-1", { type: "FixTests" });
  expect(ipc.testsAddLinked).toHaveBeenCalledWith("task-1", "proj-2");
});
test("test-started from elsewhere attaches once; a run already followed is left alone", async () => {
  vi.useFakeTimers();
  const ipc = fakeIpc(0);
  const s = createTestStore(ipc, channel);
  s.onTestFinished("task-1", 1);
  const p = s.onTestStarted("task-1");
  expect(s.store.get()["task-1"]).toMatchObject({ lines: [], code: null, running: true });
  void s.onTestStarted("task-1");
  expect(ipc.testsRun).toHaveBeenCalledTimes(1);
  await vi.runAllTimersAsync();
  await p;
  expect(s.store.get()["task-1"]).toMatchObject({ code: 0, running: false });
  vi.useRealTimers();
});
