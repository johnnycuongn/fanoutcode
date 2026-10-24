// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within, act } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings } from "@/test/mockBindings";
import { makeTask, makeProject, needsYou, IDLE } from "@/test/fixtures";
import { tasksStore, workspaceContainer } from "@/lib/stores/tasksStore";
import { projectsStore } from "@/lib/stores/projectsStore";
import { viewsStore } from "@/features/views";
import { shellStore, initWindow, toggleProjectFocus, ProjectRail } from "@/features/shell";
import { Sidebar } from "./Sidebar";

const container = workspaceContainer("ws-d4");
const ids = ["proj-a1", "proj-b2"];
describe("workspace sidebar", () => {
  beforeEach(() => {
    initWindow({ window_id: "w-2", title: "WS", container, project_ids: ids });
    shellStore.set({ sidebarOpen: true, sidebarSection: "tasks", projectLayer: "MultiRoot" });
    projectsStore.set({
      byId: {
        "proj-a1": makeProject({ id: "proj-a1", name: "Alpha", color: "violet" }),
        "proj-b2": makeProject({ id: "proj-b2", name: "Beta", color: "teal" }),
      },
      order: ids,
    });
    tasksStore.set({
      container,
      loaded: true,
      order: ["task-01", "task-02"],
      byId: {
        "task-01": makeTask({ id: "task-01", title: "A1", project_id: "proj-a1", state: needsYou() }),
        "task-02": makeTask({ id: "task-02", title: "B1", project_id: "proj-b2", state: IDLE }),
      },
    });
    viewsStore.set({ container, loaded: true, views: [], focusedPane: {}, pickerFor: null });
  });
  it("renders one collapsible root per project with its needs-you count, in order", () => {
    render(<Sidebar />);
    const roots = document.querySelectorAll("[data-project-root]");
    expect([...roots].map((r) => r.getAttribute("data-project-root"))).toEqual(ids);
    expect(within(roots[0] as HTMLElement).getByText("1")).toBeInTheDocument();
    expect(within(roots[0] as HTMLElement).getByText("A1")).toBeInTheDocument();
    expect(within(roots[1] as HTMLElement).getByText("B1")).toBeInTheDocument();
  });
  it("highlights the focused root with ⌘2 and clears on repeat", () => {
    render(<Sidebar />);
    act(() => toggleProjectFocus(2, ids));
    expect(document.querySelector('[data-project-root="proj-b2"]')).toHaveAttribute("data-focused", "true");
    act(() => toggleProjectFocus(2, ids));
    expect(document.querySelector('[data-focused="true"]')).toBeNull();
  });
  it("in rail mode shows only the focused project and offers All", () => {
    shellStore.set({ projectLayer: "Rail", focusedProject: "proj-b2" });
    render(
      <>
        <ProjectRail />
        <Sidebar />
      </>,
    );
    expect(within(screen.getByRole("tablist", { name: "Projects" })).getAllByRole("tab")).toHaveLength(3);
    expect(screen.queryByText("A1")).toBeNull();
    expect(screen.getByText("B1")).toBeInTheDocument();
  });
});
