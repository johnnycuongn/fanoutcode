// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
import { commands, events, mockBindings } from "@/test/mockBindings";
import { makeTask, makeView } from "@/test/fixtures";
import type { OverlapRow, PendingDecision } from "@/lib/bindings";
import { leaf, split } from "@/features/views";
import {
  decide,
  overlapCount,
  overlapStore,
  pendingRows,
  refreshOverlaps,
  startOverlapEvents,
  viewOverlapCount,
} from "./store";

const EDIT: PendingDecision = { type: "Edit", tool: "Edit", input: { file_path: "/r/alpha/src/a.ts" } };

function row(over: Partial<OverlapRow> = {}): OverlapRow {
  return {
    task_id: "task-b2",
    other_task: "task-c3",
    repo_root: "/r/alpha",
    path: "src/a.ts",
    relation: "Claimed",
    pending: null,
    held: false,
    noted_out: false,
    noted_in: false,
    decided_at: null,
    updated_at: "2026-10-25T08:00:00Z",
    ...over,
  };
}

const flush = () => new Promise((r) => setTimeout(r, 0));

describe("overlapStore", () => {
  beforeEach(() => {
    overlapStore.reset();
    vi.clearAllMocks();
  });

  it("refresh loads rows and counts", async () => {
    const rows = [row(), row({ path: "src/b.ts", pending: EDIT })];
    commands.overlapList.mockResolvedValue({ status: "ok", data: rows });
    await refreshOverlaps("task-b2");
    expect(commands.overlapList).toHaveBeenCalledWith("task-b2");
    expect(overlapStore.get().byTask["task-b2"]).toEqual(rows);
    expect(overlapStore.get().counts["task-b2"]).toEqual({ count: 2, pending: 1 });
    expect(overlapCount(overlapStore.get(), "task-b2")).toBe(2);
    expect(overlapCount(overlapStore.get(), "task-zz")).toBe(0);
    expect(pendingRows(rows)).toEqual([rows[1]]);
  });

  it("refresh keeps the state when the command fails", async () => {
    commands.overlapList.mockResolvedValue({ status: "error", error: "no such task" });
    await refreshOverlaps("task-b2");
    expect(overlapStore.get().byTask).toEqual({});
  });

  it("overlap-changed updates counts and refetches a loaded task", async () => {
    const off = startOverlapEvents();
    events.overlapChanged.emitForTest({ task_id: "task-c3", count: 3, pending: 0 });
    expect(overlapStore.get().counts["task-c3"]).toEqual({ count: 3, pending: 0 });
    expect(commands.overlapList).not.toHaveBeenCalled();

    commands.overlapList.mockResolvedValue({ status: "ok", data: [row()] });
    await refreshOverlaps("task-b2");
    commands.overlapList.mockClear();
    commands.overlapList.mockResolvedValue({ status: "ok", data: [row(), row({ path: "src/c.ts", pending: EDIT })] });
    events.overlapChanged.emitForTest({ task_id: "task-b2", count: 2, pending: 1 });
    expect(overlapStore.get().counts["task-b2"]).toEqual({ count: 2, pending: 1 });
    await flush();
    expect(commands.overlapList).toHaveBeenCalledWith("task-b2");
    expect(overlapStore.get().byTask["task-b2"]).toHaveLength(2);
    off();
    await flush();
    events.overlapChanged.emitForTest({ task_id: "task-c3", count: 9, pending: 9 });
    expect(overlapStore.get().counts["task-c3"]).toEqual({ count: 3, pending: 0 });
  });

  it("decide calls overlapDecide with the row's keys and refreshes", async () => {
    commands.overlapDecide.mockResolvedValue({ status: "ok", data: null });
    commands.overlapList.mockResolvedValue({ status: "ok", data: [row({ relation: "GrantedOnTop" })] });
    await decide("task-b2", row({ pending: EDIT }), { type: "OnTopFile" });
    expect(commands.overlapDecide).toHaveBeenCalledWith("task-b2", "/r/alpha", "src/a.ts", "task-c3", {
      type: "OnTopFile",
    });
    expect(commands.overlapList).toHaveBeenCalledWith("task-b2");
    expect(overlapStore.get().counts["task-b2"]).toEqual({ count: 1, pending: 0 });
  });

  it("decide rejects with the command's error and does not refresh", async () => {
    commands.overlapDecide.mockResolvedValue({ status: "error", error: "not pending" });
    await expect(decide("task-b2", row(), { type: "Hold" })).rejects.toBe("not pending");
    expect(commands.overlapList).not.toHaveBeenCalled();
  });

  it("viewOverlapCount sums distinct tasks of the view", () => {
    overlapStore.set({
      counts: { "task-b2": { count: 2, pending: 1 }, "task-c3": { count: 3, pending: 0 } },
    });
    const view = makeView({
      layout: split(
        "Horizontal",
        0.5,
        leaf("pane-aa", { type: "Task", task_id: "task-b2" }),
        split(
          "Vertical",
          0.5,
          leaf("pane-bb", { type: "TeamCanvas", task_id: "task-b2" }),
          leaf("pane-cc", { type: "Task", task_id: "task-c3" }),
        ),
      ),
    });
    const tasksById = { "task-b2": makeTask({ id: "task-b2" }), "task-c3": makeTask({ id: "task-c3" }) };
    expect(viewOverlapCount(view, tasksById, overlapStore.get().counts)).toBe(5);
    expect(viewOverlapCount(view, { "task-b2": tasksById["task-b2"] }, overlapStore.get().counts)).toBe(2);
  });
});
