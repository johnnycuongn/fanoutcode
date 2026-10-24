// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings } from "@/test/mockBindings";
import { makeTask, makeView, needsYou, WORKING, IDLE } from "@/test/fixtures";
import { tasksStore, projectContainer } from "@/lib/stores/tasksStore";
import { viewsStore, leaf, EMPTY_PANE, taskContent } from "@/features/views";
import { shellStore, initWindow, setTabOrder } from "@/features/shell";
import { Sidebar } from "./Sidebar";

const container = projectContainer("proj-a1");
describe("Sidebar", () => {
  beforeEach(() => {
    initWindow({ window_id: "w-1", title: "Alpha", container, project_ids: ["proj-a1"] });
    shellStore.set({ sidebarOpen: true, sidebarSection: "tasks", activeTab: "board" });
    tasksStore.set({
      container,
      loaded: true,
      order: ["task-01", "task-02", "task-03"],
      byId: {
        "task-01": makeTask({ id: "task-01", title: "Plan it", stage: "Plan", state: needsYou() }),
        "task-02": makeTask({ id: "task-02", title: "Building", state: WORKING }),
        "task-03": makeTask({ id: "task-03", title: "Parked", state: IDLE }),
      },
    });
    viewsStore.set({
      container,
      loaded: true,
      focusedPane: { "view-c3": "pane-aa" },
      pickerFor: null,
      views: [makeView({ id: "view-c3", name: "Main", order: 0, layout: leaf("pane-aa", EMPTY_PANE) })],
    });
    setTabOrder(["view-c3"]);
  });
  it("shows five sections in a horizontal strip and Tasks by default", () => {
    render(<Sidebar />);
    const strip = screen.getByRole("tablist", { name: "Sidebar sections" });
    expect(
      within(strip)
        .getAllByRole("tab")
        .map((t) => t.getAttribute("aria-label")),
    ).toEqual(["Tasks", "Docs", "Changes", "Ticket", "Settings"]);
    expect(screen.getByText("Needs you")).toBeInTheDocument();
    expect(within(screen.getByTestId("group-needs-you")).getByText("Plan written, go?")).toBeInTheDocument();
    expect(within(screen.getByTestId("group-running")).getByText("Building")).toBeInTheDocument();
    expect(within(screen.getByTestId("group-idle")).getByText("Parked")).toBeInTheDocument();
  });
  it("switches sections from the strip", () => {
    render(<Sidebar />);
    fireEvent.click(screen.getByRole("tab", { name: "Ticket" }));
    expect(shellStore.get().sidebarSection).toBe("ticket");
  });
  it("opens a task in the current view from the board", () => {
    render(<Sidebar />);
    fireEvent.click(screen.getByText("Plan it"));
    expect(shellStore.get().activeTab).toBe("view-c3");
    expect(viewsStore.get().views[0].layout).toEqual(leaf("pane-aa", taskContent("task-01")));
  });
  it("renders nothing visible when closed", () => {
    shellStore.set({ sidebarOpen: false });
    render(<Sidebar />);
    expect(screen.queryByRole("complementary", { name: "Sidebar" })).toBeNull();
  });
});
