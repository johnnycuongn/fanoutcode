// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import type { BoardProps } from "@/features/board";
vi.mock("@/lib/bindings", () => mockBindings());
const board = vi.hoisted(() => ({ props: null as unknown as BoardProps, openNewTask: vi.fn() }));
vi.mock("@/features/board", () => ({
  Board: (p: BoardProps) => {
    board.props = p;
    return <div data-testid="board">{String(p.focusedProject)}</div>;
  },
  openNewTask: board.openNewTask,
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ setTitle: vi.fn(), setBadgeCount: vi.fn(), isFocused: async () => true }),
}));
import { mockBindings, commands } from "@/test/mockBindings";
import { makeProject, makeView } from "@/test/fixtures";
import { projectContainer } from "@/lib/stores/tasksStore";
import { leaf, EMPTY_PANE } from "@/features/views";
import { shellStore } from "../store";
import { ShellLayout } from "./ShellLayout";

const container = projectContainer("proj-a1");
describe("ShellLayout", () => {
  beforeEach(() => {
    localStorage.clear();
    commands.windowsCurrent.mockResolvedValue({
      status: "ok",
      data: { window_id: "w-1", title: "Alpha", container, project_ids: ["proj-a1"] },
    });
    commands.projectsList.mockResolvedValue({ status: "ok", data: [makeProject({ id: "proj-a1", name: "Alpha" })] });
    commands.tasksList.mockResolvedValue({ status: "ok", data: [] });
    commands.viewsList.mockResolvedValue({
      status: "ok",
      data: [makeView({ id: "view-c3", name: "Main", order: 0, layout: leaf("pane-aa", EMPTY_PANE) })],
    });
    commands.settingsGet.mockResolvedValue({
      status: "ok",
      data: {
        theme: "System",
        detail_level: "Simple",
        cost_overview: false,
        workspace_project_layer: "MultiRoot",
        workspace_board: "Swimlanes",
        first_run_complete: true,
      },
    });
    commands.windowsFocus.mockResolvedValue({ status: "ok", data: null });
  });
  it("shows the project name, Board first, and the board by default with Plan 07's props", async () => {
    render(<ShellLayout />);
    expect(await screen.findByText("Alpha")).toBeInTheDocument();
    expect(screen.getByTestId("board")).toHaveTextContent("all");
    expect(screen.getAllByRole("tab")[0]).toHaveTextContent("Board");
    expect(board.props).toMatchObject({ container, focusedProject: "all", currentWindowId: "w-1" });
    expect(board.props.projects.map((p) => p.id)).toEqual(["proj-a1"]);
    expect(board.props.settings).toBeTruthy();
    board.props.onNewTask();
    expect(board.openNewTask).toHaveBeenCalledTimes(1);
    board.props.onOpenTask("task-b2", { kind: "window", window_id: "w-2" });
    expect(commands.windowsFocus).toHaveBeenCalledWith("w-2");
  });
  it("switches to a view with ⌘2 and back to the board with ⌘B, keeping the view mounted", async () => {
    render(<ShellLayout />);
    await screen.findByRole("tab", { name: /Main/ });
    fireEvent.keyDown(window, { key: "2", metaKey: true });
    expect(shellStore.get().activeTab).toBe("view-c3");
    expect(screen.getByRole("button", { name: "Pick a task" })).toBeVisible();
    fireEvent.keyDown(window, { key: "b", metaKey: true });
    expect(shellStore.get().activeTab).toBe("board");
    expect(screen.getByRole("button", { name: "Pick a task", hidden: true })).not.toBeVisible();
  });
  it("toggles the sidebar with ⌘\\", async () => {
    render(<ShellLayout />);
    await screen.findByText("Alpha");
    const before = shellStore.get().sidebarOpen;
    fireEvent.keyDown(window, { key: "\\", metaKey: true });
    expect(shellStore.get().sidebarOpen).toBe(!before);
  });
});
