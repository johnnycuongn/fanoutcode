// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { TaskPageHandle } from "@/features/task-page";
vi.mock("@/lib/bindings", () => mockBindings());
vi.mock("@/features/board", () => ({ Board: () => null, openNewTask: vi.fn() }));
import { mockBindings, commands } from "@/test/mockBindings";
import { makeTask, makeView } from "@/test/fixtures";
import { tasksStore, projectContainer } from "@/lib/stores/tasksStore";
import { viewsStore, setPaneHandle, leaf, taskContent } from "@/features/views";
import { shellStore, initWindow, setTabOrder } from "@/features/shell";
import { paletteStore, openPalette } from "../store";
import { CommandPalette } from "./CommandPalette";

const container = projectContainer("proj-a1");
describe("CommandPalette", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setPaneHandle("pane-aa", null);
    initWindow({ window_id: "w-1", title: "Alpha", container, project_ids: ["proj-a1"] });
    tasksStore.set({
      container,
      loaded: true,
      order: ["task-b2"],
      byId: { "task-b2": makeTask({ id: "task-b2", title: "Login flow", stage: "Plan" }) },
    });
    viewsStore.set({
      container,
      loaded: true,
      focusedPane: { "view-c3": "pane-aa" },
      pickerFor: null,
      views: [makeView({ id: "view-c3", order: 0, layout: leaf("pane-aa", taskContent("task-b2")) })],
    });
    setTabOrder(["view-c3"]);
    shellStore.set({ activeTab: "view-c3" });
    commands.windowsList.mockResolvedValue([]);
    commands.tasksAction.mockResolvedValue({ status: "ok", data: { task: makeTask(), toasts: [] } });
    openPalette();
  });
  it("runs the focused task's primary action and closes", async () => {
    render(<CommandPalette />);
    await userEvent.type(screen.getByRole("combobox"), "start bu{enter}");
    expect(commands.tasksAction).toHaveBeenCalledWith("task-b2", { type: "Advance" });
    expect(paletteStore.get().open).toBe(false);
  });
  it("Reply focuses the task terminal instead of sending a prompt", async () => {
    const handle = { focusTerminal: vi.fn() } as unknown as TaskPageHandle;
    setPaneHandle("pane-aa", { kind: "task", handle });
    render(<CommandPalette />);
    await userEvent.type(screen.getByRole("combobox"), "reply{enter}");
    expect(handle.focusTerminal).toHaveBeenCalledTimes(1);
    expect(commands.tasksAction).not.toHaveBeenCalled();
  });
  it("goes to a task", async () => {
    render(<CommandPalette />);
    await userEvent.type(screen.getByRole("combobox"), "login{enter}");
    expect(viewsStore.get().views[0].layout).toEqual(leaf("pane-aa", taskContent("task-b2")));
  });
});
