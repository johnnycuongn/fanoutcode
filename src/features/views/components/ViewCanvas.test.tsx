// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import type { Ref } from "react";
import type { TaskPageHandle, TaskPageProps } from "@/features/task-page";
const handle = vi.hoisted(() => ({
  primaryAction: vi.fn(),
  rename: vi.fn(),
  archive: vi.fn(),
  toggleDock: vi.fn(),
  growTeam: vi.fn(),
  toggleTerminalStrip: vi.fn(),
  focusTerminal: vi.fn(),
}));
const seen = vi.hoisted(() => ({ props: null as unknown as TaskPageProps, canvas: null as unknown }));
vi.mock("@/lib/bindings", () => mockBindings());
vi.mock("@/features/task-page", async () => {
  const { useImperativeHandle } = await import("react");
  return {
    TaskPage: ({ ref, ...props }: TaskPageProps & { ref?: Ref<TaskPageHandle> }) => {
      seen.props = props;
      useImperativeHandle(ref, () => handle);
      return <div data-testid="task-page">{props.taskId}</div>;
    },
  };
});
vi.mock("@/features/canvas", () => ({
  CanvasPane: (p: { taskId: string }) => {
    seen.canvas = p;
    return <div data-testid="canvas-pane">{p.taskId}</div>;
  },
}));
import { mockBindings } from "@/test/mockBindings";
import { makeTask, makeProject } from "@/test/fixtures";
import { tasksStore, projectContainer } from "@/lib/stores/tasksStore";
import { projectsStore } from "@/lib/stores/projectsStore";
import { matchShortcut } from "@/lib/keyboard";
import { shellStore, initWindow } from "@/features/shell";
import { viewsStore, splitFocused } from "../store";
import { leaf, split, taskContent } from "../layout";
import { ViewCanvas } from "./ViewCanvas";

const container = projectContainer("proj-a1");
const twoPanes = split(
  "Horizontal",
  0.5,
  leaf("pane-aa", taskContent("task-b2")),
  leaf("pane-bb", taskContent("task-gone")),
);
const press = (key: string, m: KeyboardEventInit = {}, scope: "pane" | "spec-page" = "pane") =>
  act(() => {
    matchShortcut(
      new KeyboardEvent("keydown", { key, metaKey: true, ...m }),
      { scope, terminalFocused: false },
      { dispatch: true },
    );
  });

describe("ViewCanvas", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    initWindow({ window_id: "w-1", title: "Alpha", container, project_ids: ["proj-a1"] });
    shellStore.set({ activeTab: "view-c3", tabOrder: ["board", "view-c3"] });
    projectsStore.set({ byId: { "proj-a1": makeProject({ id: "proj-a1" }) }, order: ["proj-a1"] });
    tasksStore.set({
      container,
      byId: { "task-b2": makeTask({ id: "task-b2", title: "Alpha task" }) },
      order: ["task-b2"],
      loaded: true,
    });
    viewsStore.set({
      container,
      views: [{ id: "view-c3", container, name: "V", order: 0, layout: twoPanes }],
      focusedPane: { "view-c3": "pane-aa" },
      pickerFor: null,
      loaded: true,
    });
  });
  it("renders a task page with Plan 08's props and a placeholder for a missing task", async () => {
    render(<ViewCanvas viewId="view-c3" />);
    expect(await screen.findByTestId("task-page")).toHaveTextContent("task-b2");
    expect(seen.props).toMatchObject({ taskId: "task-b2", paneId: "pane-aa", container });
    expect(seen.props.projects.map((p) => p.id)).toEqual(["proj-a1"]);
    expect(screen.getByText("Task no longer available")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Pick another task" })).toBeInTheDocument();
    act(() => seen.props.onBack());
    expect(shellStore.get().activeTab).toBe("board");
  });
  it("marks the focused pane and moves focus on click", () => {
    render(<ViewCanvas viewId="view-c3" />);
    const panes = document.querySelectorAll("[data-pane-id]");
    expect(panes[0].getAttribute("data-focused")).toBe("true");
    fireEvent.click(panes[1]);
    expect(viewsStore.get().focusedPane["view-c3"]).toBe("pane-bb");
  });
  it("closes a pane from its header button", () => {
    render(<ViewCanvas viewId="view-c3" />);
    fireEvent.click(screen.getAllByRole("button", { name: "Close pane" })[1]);
    expect(document.querySelectorAll("[data-pane-id]")).toHaveLength(1);
  });
  it("shows the Empty state with a picker button after a split", () => {
    render(<ViewCanvas viewId="view-c3" />);
    act(() => splitFocused("Vertical"));
    expect(screen.getByRole("button", { name: "Pick a task" })).toBeInTheDocument();
  });
  it("routes the task chords to the focused task page and focuses its terminal on ⌘⌥ arrows", async () => {
    render(<ViewCanvas viewId="view-c3" />);
    await screen.findByTestId("task-page");
    press("Enter");
    press("e");
    press("A", { shiftKey: true });
    press("j");
    press("J", { shiftKey: true });
    press("`", { metaKey: false, ctrlKey: true }, "spec-page");
    for (const fn of ["primaryAction", "rename", "archive", "toggleDock", "growTeam", "toggleTerminalStrip"] as const)
      expect(handle[fn]).toHaveBeenCalledTimes(1);
    press("ArrowRight", { altKey: true });
    expect(viewsStore.get().focusedPane["view-c3"]).toBe("pane-bb");
    press("Enter");
    expect(handle.primaryAction).toHaveBeenCalledTimes(1);
    press("ArrowLeft", { altKey: true });
    expect(handle.focusTerminal).toHaveBeenCalledTimes(1);
  });
  it("opens the team canvas pane through onOpenTeamPane and renders CanvasPane", async () => {
    render(<ViewCanvas viewId="view-c3" />);
    await screen.findByTestId("task-page");
    act(() => seen.props.onOpenTeamPane?.(makeTask({ id: "task-b2" })));
    expect(await screen.findByTestId("canvas-pane")).toHaveTextContent("task-b2");
    expect(seen.canvas).toEqual({ taskId: "task-b2" });
  });
});
