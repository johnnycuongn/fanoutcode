// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands, events } from "@/test/mockBindings";
import { makeTask, makeView, WORKING, IDLE } from "@/test/fixtures";
import { tasksStore, projectContainer } from "@/lib/stores/tasksStore";
import { viewsStore, leaf, EMPTY_PANE, taskContent } from "@/features/views";
import { shellStore, initWindow, setTabOrder } from "@/features/shell";
import { overlapStore } from "@/features/overlap";
import type { OverlapRow } from "@/lib/bindings";
import { ChangesSection } from "./ChangesSection";

const overlap: OverlapRow = {
  task_id: "task-01",
  other_task: "task-02",
  repo_root: "/r/alpha",
  path: "src/a.ts",
  relation: "Claimed",
  pending: null,
  held: false,
  noted_out: false,
  noted_in: false,
  decided_at: null,
  updated_at: "2026-10-25T08:00:00Z",
};

const container = projectContainer("proj-a1");
const changes = [{ repo_root: "/r/alpha", files: [{ path: "src/a.ts", added: 4, removed: 1 }] }];
describe("ChangesSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    initWindow({ window_id: "w-1", title: "Alpha", container, project_ids: ["proj-a1"] });
    tasksStore.set({
      container,
      loaded: true,
      order: ["task-01", "task-02"],
      byId: {
        "task-01": makeTask({ id: "task-01", title: "Active", stage: "Build", state: WORKING }),
        "task-02": makeTask({ id: "task-02", title: "Parked", stage: "Spec", state: IDLE }),
      },
    });
    viewsStore.set({
      container,
      loaded: true,
      focusedPane: { "view-c3": "pane-aa" },
      pickerFor: null,
      views: [makeView({ id: "view-c3", order: 0, layout: leaf("pane-aa", EMPTY_PANE) })],
    });
    setTabOrder(["view-c3"]);
    commands.gitChanges.mockResolvedValue({ status: "ok", data: changes });
    commands.overlapList.mockResolvedValue({ status: "ok", data: [] });
    overlapStore.set({ byTask: {}, counts: {} });
  });
  it("changes section marks a file another task touches", async () => {
    commands.overlapList.mockResolvedValue({ status: "ok", data: [overlap] });
    render(<ChangesSection />);
    await screen.findByText("src/a.ts");
    expect(commands.overlapList).toHaveBeenCalledWith("task-01");
    expect(commands.overlapList).not.toHaveBeenCalledWith("task-02");
    const fileButton = screen.getByText("src/a.ts").closest("button")!;
    await vi.waitFor(() => expect(fileButton).toHaveTextContent("claimed"));
  });
  it("leaves a file no other task touches unmarked", async () => {
    render(<ChangesSection />);
    await screen.findByText("src/a.ts");
    await vi.waitFor(() => expect(commands.overlapList).toHaveBeenCalled());
    expect(screen.queryByText("claimed")).toBeNull();
  });
  it("lists files grouped by active task only", async () => {
    render(<ChangesSection />);
    expect(await screen.findByText("src/a.ts")).toBeInTheDocument();
    expect(screen.getByText("+4 −1")).toBeInTheDocument();
    expect(commands.gitChanges).toHaveBeenCalledTimes(1);
    expect(commands.gitChanges).toHaveBeenCalledWith("task-01");
    expect(screen.queryByText("Parked")).toBeNull();
  });
  it("opens the task with the Changes dock tab requested", async () => {
    render(<ChangesSection />);
    fireEvent.click(await screen.findByText("src/a.ts"));
    expect(viewsStore.get().views[0].layout).toEqual(leaf("pane-aa", taskContent("task-01")));
    expect(shellStore.get().pendingDockTab).toEqual({ taskId: "task-01", tab: "changes" });
  });
  it("refetches on git-changes for that task", async () => {
    render(<ChangesSection />);
    await screen.findByText("src/a.ts");
    events.gitChanges.emitForTest({ task_id: "task-01" });
    await vi.waitFor(() => expect(commands.gitChanges).toHaveBeenCalledTimes(2));
  });
});
