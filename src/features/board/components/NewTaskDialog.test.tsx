import { act, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("../ipc");
vi.mock("@/components/ui/EmptyStateLottie", () => ({ default: () => null })); // jsdom has no WebAssembly
import { closeNewTask, openNewTask } from "../newTask";
import { resetBoardStore } from "../store";
import { taskMap } from "../taskCache";
import { makeProject, makeTask, projectContainer, workspaceContainer } from "../testing/fixtures";
import { mockIpc } from "../testing/mockIpc";
import { mountUi } from "../testing/render";
import { Board } from "./Board";

const settings = {
  detailLevel: "Simple",
  detailLevelByProject: {},
  costOverview: false,
  workspaceBoard: "Swimlanes",
  projectLayer: "MultiRoot",
} as const;
const base = { settings, focusedProject: null, currentWindowId: "w-1", onOpenTask: vi.fn(), onNewTask: vi.fn() };

describe("new task", () => {
  let m: ReturnType<typeof mockIpc>;
  beforeEach(() => {
    resetBoardStore();
    closeNewTask();
    m = mockIpc();
  });

  it("openNewTask opens the sheet and Create calls tasks_create for the project", async () => {
    m.ipc.tasksCreate.mockResolvedValue(makeTask({ id: "t-9", title: "Add login" }));
    mountUi(<Board {...base} container={projectContainer("p-1")} projects={[makeProject({ id: "p-1" })]} />);
    await screen.findByText("No tasks yet");
    act(() => openNewTask());
    await userEvent.type(await screen.findByLabelText("What should be built?"), "Add login");
    await userEvent.type(screen.getByLabelText("Jira issue (optional)"), "tc-4");
    await userEvent.click(screen.getByRole("button", { name: /Create task/ }));
    expect(m.ipc.tasksCreate).toHaveBeenCalledWith("p-1", "Add login", "TC-4", null);
    expect(taskMap()["t-9"]).toBeDefined();
    await vi.waitFor(() =>
      expect(screen.queryByText("Describe it in your own words; the agent starts from the spec.")).toBeNull(),
    );
  });
  it("asks for a description and a well-formed Jira key before calling the core", async () => {
    mountUi(<Board {...base} container={projectContainer("p-1")} projects={[makeProject({ id: "p-1" })]} />);
    await screen.findByText("No tasks yet");
    act(() => openNewTask());
    await userEvent.click(await screen.findByRole("button", { name: /Create task/ }));
    expect(screen.getByText("Describe what you want built")).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("What should be built?"), "x");
    await userEvent.type(screen.getByLabelText("Jira issue (optional)"), "nope");
    await userEvent.click(screen.getByRole("button", { name: /Create task/ }));
    expect(screen.getByText("Use the form PROJ-123 for the Jira issue")).toBeInTheDocument();
    expect(m.ipc.tasksCreate).not.toHaveBeenCalled();
  });
  it("in a workspace the task goes to the picked project with the workspace id", async () => {
    m.ipc.tasksCreate.mockResolvedValue(makeTask({ id: "t-9", project_id: "p-1" }));
    mountUi(
      <Board
        {...base}
        container={workspaceContainer("ws-1")}
        projects={[makeProject({ id: "p-1", name: "Alpha" }), makeProject({ id: "p-2", name: "Beta" })]}
        focusedProject="all"
      />,
    );
    await screen.findByText("No tasks yet");
    act(() => openNewTask());
    await userEvent.type(await screen.findByLabelText("What should be built?"), "Do it");
    await userEvent.click(screen.getByRole("button", { name: /Create task/ }));
    expect(m.ipc.tasksCreate).toHaveBeenCalledWith("p-1", "Do it", null, "ws-1");
  });
  it("a refused create keeps the sheet open", async () => {
    m.ipc.tasksCreate.mockRejectedValue(new Error("Precondition: no memory folder"));
    mountUi(<Board {...base} container={projectContainer("p-1")} projects={[makeProject({ id: "p-1" })]} />);
    await screen.findByText("No tasks yet");
    act(() => openNewTask());
    await userEvent.type(await screen.findByLabelText("What should be built?"), "Do it");
    await userEvent.click(screen.getByRole("button", { name: /Create task/ }));
    expect(await screen.findByLabelText("What should be built?")).toHaveValue("Do it");
  });
});
