// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { Profiler } from "react";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands } from "@/test/mockBindings";
import { makeTask, makeView, needsYou } from "@/test/fixtures";
import { tasksStore, projectContainer } from "@/lib/stores/tasksStore";
import { viewsStore, leaf, split, EMPTY_PANE, taskContent, reorderViews } from "@/features/views";
import { shellStore, initWindow, setTabOrder } from "../store";
import { overlapStore } from "@/features/overlap";
import { TabStrip } from "./TabStrip";

const container = projectContainer("proj-a1");
describe("TabStrip", () => {
  beforeEach(() => {
    initWindow({ window_id: "w-1", title: "Alpha", container, project_ids: ["proj-a1"] });
    tasksStore.set({
      container,
      byId: { "task-b2": makeTask({ id: "task-b2", state: needsYou({ type: "PermissionNeeded" }) }) },
      order: ["task-b2"],
      loaded: true,
    });
    viewsStore.set({
      container,
      loaded: true,
      focusedPane: {},
      pickerFor: null,
      views: [
        makeView({ id: "view-c3", name: "Auth", order: 0, layout: leaf("pane-aa", taskContent("task-b2")) }),
        makeView({ id: "view-c4", name: "Docs", order: 1, layout: leaf("pane-bb", EMPTY_PANE) }),
      ],
    });
    setTabOrder(["view-c3", "view-c4"]);
    overlapStore.set({ byTask: {}, counts: {} });
  });
  it("view tab tooltip carries the overlap count", () => {
    overlapStore.set({ byTask: {}, counts: { "task-b2": { count: 2, pending: 0 } } });
    const { unmount } = render(<TabStrip />);
    expect(screen.getByRole("tab", { name: /Auth/ })).toHaveAttribute("title", "Spec · Needs you · 2 overlaps");
    unmount();
    overlapStore.set({ byTask: {}, counts: { "task-b2": { count: 1, pending: 1 } } });
    render(<TabStrip />);
    expect(screen.getByRole("tab", { name: /Auth/ })).toHaveAttribute("title", "Spec · Needs you · 1 overlap");
  });
  it("does not re-render when only an overlap row changes", () => {
    const commits = vi.fn();
    render(
      <Profiler id="strip" onRender={commits}>
        <TabStrip />
      </Profiler>,
    );
    const before = commits.mock.calls.length;
    act(() => overlapStore.set((s) => ({ ...s, byTask: { "task-b2": [] } })));
    expect(commits.mock.calls.length).toBe(before);
    act(() => overlapStore.set((s) => ({ ...s, counts: { "task-b2": { count: 3, pending: 0 } } })));
    expect(screen.getByRole("tab", { name: /Auth/ })).toHaveAttribute("title", "Spec · Needs you · 3 overlaps");
  });
  it("renders Board first, then views, with an amber count on the view that needs you", () => {
    render(<TabStrip />);
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((t) => t.textContent)).toEqual(["Board", "Auth1", "Docs"]);
    expect(tabs[1].querySelector("[data-tone=warn]")).toHaveTextContent("1");
  });
  it("activates a view on click and marks aria-selected", () => {
    render(<TabStrip />);
    fireEvent.click(screen.getByRole("tab", { name: /Docs/ }));
    expect(shellStore.get().activeTab).toBe("view-c4");
    expect(screen.getByRole("tab", { name: /Docs/ })).toHaveAttribute("aria-selected", "true");
  });
  it("creates a view from the + button", async () => {
    commands.viewsCreate.mockResolvedValue({
      status: "ok",
      data: makeView({ id: "view-c5", name: "View 3", order: 2 }),
    });
    render(<TabStrip />);
    fireEvent.click(screen.getByRole("button", { name: "New view" }));
    await vi.waitFor(() => expect(commands.viewsCreate).toHaveBeenCalledWith(container, "View 3"));
  });
  it("renames inline on double-click", async () => {
    commands.viewsRename.mockResolvedValue({ status: "ok", data: null });
    render(<TabStrip />);
    fireEvent.doubleClick(screen.getByRole("tab", { name: /Auth/ }));
    const input = screen.getByRole("textbox");
    fireEvent.change(input, { target: { value: "Login" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await vi.waitFor(() => expect(commands.viewsRename).toHaveBeenCalledWith("view-c3", "Login"));
  });
  it("beats a status dot and shows the stage icon on a view with tasks, and neither on an empty view or Board", () => {
    render(<TabStrip />);
    const [board, auth, docs] = screen.getAllByRole("tab");
    const dot = auth.querySelector("[data-pulse]");
    expect(dot).toHaveAttribute("data-pulse", "true");
    expect(auth.querySelector("[data-stage]")).toHaveAttribute("data-stage", "spec");
    expect(auth).toHaveAttribute("title", "Spec · Needs you");
    for (const tab of [board, docs]) {
      expect(tab.querySelector("[data-pulse]")).toBeNull();
      expect(tab.querySelector("[data-stage]")).toBeNull();
    }
  });
  it("shows the several-stages icon when a view's tasks sit in more than one stage", () => {
    tasksStore.set({
      ...tasksStore.get(),
      byId: {
        "task-b2": makeTask({ id: "task-b2", stage: "Spec" }),
        "task-b3": makeTask({ id: "task-b3", stage: "Build" }),
      },
    });
    viewsStore.set({
      ...viewsStore.get(),
      views: [
        makeView({
          id: "view-c3",
          name: "Auth",
          layout: split(
            "Horizontal",
            0.5,
            leaf("pane-aa", taskContent("task-b2")),
            leaf("pane-ab", taskContent("task-b3")),
          ),
        }),
      ],
    });
    render(<TabStrip />);
    const auth = screen.getByRole("tab", { name: /Auth/ });
    expect(auth.querySelector("[data-stage]")).toHaveAttribute("data-stage", "many");
    expect(auth).toHaveAttribute("title", "Several stages · Idle");
    expect(auth.querySelector("[data-pulse]")).toHaveAttribute("data-pulse", "false");
  });
  it("gives Board a kanban icon beside its label", () => {
    render(<TabStrip />);
    const board = screen.getByRole("tab", { name: "Board" });
    expect(board).toBe(screen.getAllByRole("tab")[0]);
    expect(board.querySelector("svg")).not.toBeNull();
    expect(board).toHaveAttribute("aria-controls", "tc-board");
  });
  it("keeps Board first and fixed when the views are reordered", async () => {
    commands.viewsReorder.mockResolvedValue({ status: "ok", data: null });
    render(<TabStrip />);
    await act(() => reorderViews(["view-c4", "view-c3"]));
    expect(screen.getAllByRole("tab").map((t) => t.textContent)).toEqual(["Board", "Docs", "Auth1"]);
  });
  it("offers Board no rename or close, by context menu or double-click", () => {
    render(<TabStrip />);
    // Control: a view tab does open the menu, so the absence below means something.
    fireEvent.contextMenu(screen.getByRole("tab", { name: /Docs/ }));
    expect(screen.getByRole("menuitem", { name: /Rename/ })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: /Close view/ })).toBeInTheDocument();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    const board = screen.getByRole("tab", { name: /Board/ });
    fireEvent.contextMenu(board);
    expect(screen.queryByRole("menuitem", { name: /Rename/ })).toBeNull();
    expect(screen.queryByRole("menuitem", { name: /Close view/ })).toBeNull();
    fireEvent.doubleClick(board);
    expect(screen.queryByRole("textbox")).toBeNull();
  });
});
