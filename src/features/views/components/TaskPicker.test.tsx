// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands } from "@/test/mockBindings";
import { makeTask, makeProject } from "@/test/fixtures";
import { tasksStore, workspaceContainer } from "@/lib/stores/tasksStore";
import { projectsStore } from "@/lib/stores/projectsStore";
import { shellStore } from "@/features/shell";
import { viewsStore } from "../store";
import { leaf, EMPTY_PANE, taskContent, terminalContent } from "../layout";
import { TaskPicker } from "./TaskPicker";

const container = workspaceContainer("ws-d4");
describe("TaskPicker", () => {
  beforeEach(() => {
    shellStore.set({ kind: "workspace", container });
    projectsStore.set({
      byId: {
        "proj-a1": makeProject({ id: "proj-a1", name: "Alpha" }),
        "proj-b2": makeProject({ id: "proj-b2", name: "Beta" }),
      },
      order: ["proj-a1", "proj-b2"],
    });
    tasksStore.set({
      container,
      byId: {
        "task-b2": makeTask({ id: "task-b2", title: "Login flow", project_id: "proj-b2" }),
        "task-b3": makeTask({ id: "task-b3", title: "Billing", project_id: "proj-a1", jira_key: "TC-7" }),
      },
      order: ["task-b2", "task-b3"],
      loaded: true,
    });
    viewsStore.set({
      container,
      views: [{ id: "view-c3", container, name: "V", order: 0, layout: leaf("pane-aa", EMPTY_PANE) }],
      focusedPane: {},
      pickerFor: { viewId: "view-c3", pane: "pane-aa" },
      loaded: true,
    });
    commands.viewsUpdateLayout.mockResolvedValue({ status: "ok", data: null });
  });
  it("lists tasks with project badges and two terminal entries", () => {
    render(<TaskPicker />);
    expect(screen.getByText("Login flow").closest("[role=option]")).toHaveTextContent("Beta");
    expect(screen.getByText("New terminal in Alpha")).toBeInTheDocument();
    expect(screen.getByText("New terminal in Beta")).toBeInTheDocument();
  });
  it("filters by Jira key and fills the pane on Enter", async () => {
    render(<TaskPicker />);
    await userEvent.type(screen.getByRole("combobox"), "tc-7{enter}");
    expect(viewsStore.get().views[0].layout).toEqual(leaf("pane-aa", taskContent("task-b3")));
    expect(viewsStore.get().pickerFor).toBeNull();
  });
  it("opens a terminal in the chosen project", async () => {
    commands.sessionsTerminal.mockResolvedValue({ status: "ok", data: "sess-9f" });
    render(<TaskPicker />);
    await userEvent.click(screen.getByText("New terminal in Alpha"));
    expect(commands.sessionsTerminal).toHaveBeenCalledWith("proj-a1");
    await vi.waitFor(() =>
      expect(viewsStore.get().views[0].layout).toEqual(leaf("pane-aa", terminalContent("proj-a1", "sess-9f"))),
    );
  });
});
