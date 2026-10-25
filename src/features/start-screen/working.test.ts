import { beforeEach, expect, it, vi } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands, events } from "@/test/mockBindings";
import { makeTask, WORKING } from "@/test/fixtures";
import { loadWorkingCount, startWorkingEvents, workingStore } from "./working";

beforeEach(() => {
  workingStore.reset();
  commands.tasksWorkingCount.mockReset();
});

it("loads the count of working tasks across every project", async () => {
  commands.tasksWorkingCount.mockResolvedValue({ status: "ok", data: 2 });
  await loadWorkingCount();
  expect(workingStore.get().count).toBe(2);
});

it("keeps the last count when the command fails", async () => {
  workingStore.set({ count: 1 });
  commands.tasksWorkingCount.mockResolvedValue({ status: "error", error: "db" });
  await loadWorkingCount();
  expect(workingStore.get().count).toBe(1);
});

it("refreshes on task-changed and tasks-reloaded, and stops when told", async () => {
  commands.tasksWorkingCount.mockResolvedValue({ status: "ok", data: 0 });
  const stop = startWorkingEvents();
  await vi.waitFor(() => expect(commands.tasksWorkingCount).toHaveBeenCalledTimes(1));
  commands.tasksWorkingCount.mockResolvedValue({ status: "ok", data: 1 });
  events.taskChanged.emitForTest({ task: makeTask({ state: WORKING }) });
  await vi.waitFor(() => expect(workingStore.get().count).toBe(1));
  commands.tasksWorkingCount.mockResolvedValue({ status: "ok", data: 3 });
  events.tasksReloaded.emitForTest({ container: { type: "Project", id: "proj-a1" } });
  await vi.waitFor(() => expect(workingStore.get().count).toBe(3));
  stop();
  await Promise.resolve();
  events.taskChanged.emitForTest({ task: makeTask({ state: WORKING }) });
  expect(commands.tasksWorkingCount).toHaveBeenCalledTimes(3);
});

it("a slow early answer never overwrites a later one", async () => {
  let answerFirst: (v: unknown) => void = () => {};
  commands.tasksWorkingCount
    .mockImplementationOnce(() => new Promise((resolve) => (answerFirst = resolve)))
    .mockResolvedValueOnce({ status: "ok", data: 0 });
  const first = loadWorkingCount();
  await loadWorkingCount();
  answerFirst({ status: "ok", data: 4 });
  await first;
  expect(workingStore.get().count).toBe(0);
});
