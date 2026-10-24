// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands } from "@/test/mockBindings";
import { makeTask, makeView, needsYou } from "@/test/fixtures";
import { tasksStore, projectContainer } from "@/lib/stores/tasksStore";
import { viewsStore, leaf, EMPTY_PANE, taskContent } from "@/features/views";
import { shellStore, initWindow, setTabOrder } from "../store";
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
});
