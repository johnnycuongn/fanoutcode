import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("./ipc");
vi.mock("@/components/ui/Toast", () => ({ toast: { error: vi.fn(), success: vi.fn(), info: vi.fn() } }));
import { toast } from "@/components/ui/Toast";
import { GLOW_MS } from "@/lib/motion";
import { archiveTask, moveTask, renameTask, restoreTask, stopAgent, updateTask } from "./actions";
import { boardStore, resetBoardStore, upsertTask } from "./store";
import { taskMap } from "./taskCache";
import { WORKING, makeTask } from "./testing/fixtures";
import { mockIpc } from "./testing/mockIpc";

describe("actions", () => {
  let m: ReturnType<typeof mockIpc>;
  beforeEach(() => {
    resetBoardStore();
    m = mockIpc();
    vi.mocked(toast.error).mockClear();
    vi.mocked(toast.info).mockClear();
    vi.useFakeTimers();
  });

  it("moveTask sets the stage locally, marks the drop, and calls tasks_move", async () => {
    upsertTask(makeTask({ id: "t-1", stage: "Spec" }));
    m.ipc.tasksMove.mockResolvedValue(undefined as never);
    const p = moveTask("t-1", "Build");
    expect(taskMap()["t-1"]!.stage).toBe("Build");
    expect(boardStore.get().lastDrop).toMatchObject({ taskId: "t-1", stage: "Build" });
    await p;
    expect(m.ipc.tasksMove).toHaveBeenCalledWith("t-1", "Build");
  });
  it("moveTask reverts the stage and toasts when tasks_move rejects", async () => {
    // Review Focus 2
    upsertTask(makeTask({ id: "t-1", stage: "Review" }));
    m.ipc.tasksMove.mockRejectedValue(new Error("Precondition: no test command"));
    await moveTask("t-1", "Test");
    expect(taskMap()["t-1"]!.stage).toBe("Review");
    expect(boardStore.get().lastDrop).toBeNull();
    expect(toast.error).toHaveBeenCalledWith("Precondition: no test command");
  });
  it("a rejected move keeps fields that changed meanwhile", async () => {
    upsertTask(makeTask({ id: "t-1", stage: "Review", title: "Old" }));
    m.ipc.tasksMove.mockImplementation(async () => {
      upsertTask({ ...taskMap()["t-1"]!, title: "Renamed meanwhile" });
      throw new Error("no");
    });
    await moveTask("t-1", "Test");
    expect(taskMap()["t-1"]).toMatchObject({ stage: "Review", title: "Renamed meanwhile" });
  });
  it("moving to the stage a card is already in does nothing", async () => {
    upsertTask(makeTask({ id: "t-1", stage: "Plan" }));
    await moveTask("t-1", "Plan");
    expect(m.ipc.tasksMove).not.toHaveBeenCalled();
  });
  it("moveTask applies the returned task and shows the core's toasts", async () => {
    upsertTask(makeTask({ id: "t-1", stage: "Spec" }));
    m.ipc.tasksMove.mockResolvedValue({
      task: makeTask({ id: "t-1", stage: "Build", title: "From core" }),
      toasts: ["Skipped Plan"],
    });
    await moveTask("t-1", "Build");
    expect(taskMap()["t-1"]!.title).toBe("From core");
    expect(toast.info).toHaveBeenCalledWith("Skipped Plan");
  });
  it("the drop mark clears after GLOW_MS", async () => {
    upsertTask(makeTask({ id: "t-1" }));
    m.ipc.tasksMove.mockResolvedValue(undefined as never);
    await moveTask("t-1", "Plan");
    vi.advanceTimersByTime(GLOW_MS - 1);
    expect(boardStore.get().lastDrop).not.toBeNull();
    vi.advanceTimersByTime(1);
    expect(boardStore.get().lastDrop).toBeNull();
  });
  it("archiveTask confirms first when the agent is working and skips confirmation otherwise", async () => {
    upsertTask(makeTask({ id: "w", state: WORKING }));
    upsertTask(makeTask({ id: "i" }));
    const no = vi.fn(async () => false);
    const yes = vi.fn(async () => true);
    await archiveTask("w", no);
    expect(m.ipc.tasksArchive).not.toHaveBeenCalled();
    await archiveTask("i", no);
    expect(no).toHaveBeenCalledTimes(1);
    expect(m.ipc.tasksArchive).toHaveBeenCalledWith("i");
    await archiveTask("w", yes);
    expect(m.ipc.tasksArchive).toHaveBeenCalledWith("w");
  });
  it("restoreTask calls tasks_restore and toasts a failure", async () => {
    upsertTask(makeTask({ id: "a", archived: true }));
    await restoreTask("a");
    expect(m.ipc.tasksRestore).toHaveBeenCalledWith("a");
    m.ipc.tasksRestore.mockRejectedValue(new Error("gone"));
    await restoreTask("a");
    expect(toast.error).toHaveBeenCalledWith("gone");
  });
  it("stopAgent calls sessions_stop with the primary session after confirmation", async () => {
    upsertTask(makeTask({ id: "t-1", state: WORKING, primary_session: "s-9" }));
    await stopAgent("t-1", async () => false);
    expect(m.ipc.sessionsStop).not.toHaveBeenCalled();
    await stopAgent("t-1", async () => true);
    expect(m.ipc.sessionsStop).toHaveBeenCalledWith("s-9");
  });
  it("renameTask trims and ignores an empty title", async () => {
    upsertTask(makeTask({ id: "t-1" }));
    await renameTask("t-1", "   ");
    expect(m.ipc.tasksRename).not.toHaveBeenCalled();
    await renameTask("t-1", "  New name ");
    expect(m.ipc.tasksRename).toHaveBeenCalledWith("t-1", "New name");
  });
  it("updateTask sends the patch and stores the returned task", async () => {
    upsertTask(makeTask({ id: "t-1" }));
    m.ipc.tasksUpdate.mockResolvedValue(makeTask({ id: "t-1", flagged: true }));
    await updateTask("t-1", { flagged: true });
    expect(m.ipc.tasksUpdate).toHaveBeenCalledWith("t-1", { flagged: true });
    expect(taskMap()["t-1"]!.flagged).toBe(true);
  });
});
