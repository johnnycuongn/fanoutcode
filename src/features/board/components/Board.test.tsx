import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("../ipc");
vi.mock("@/components/ui/EmptyStateLottie", () => ({ default: () => null })); // jsdom has no WebAssembly
import { resetBoardStore } from "../store";
import { makeProject, makeTask, needsYou, projectContainer } from "../testing/fixtures";
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
const props = {
  container: projectContainer("p-1"),
  projects: [makeProject({ id: "p-1" })],
  settings,
  focusedProject: null,
  currentWindowId: "w-1",
  onOpenTask: vi.fn(),
  onNewTask: vi.fn(),
};

describe("Board", () => {
  let m: ReturnType<typeof mockIpc>;
  beforeEach(() => {
    resetBoardStore();
    m = mockIpc();
    m.ipc.windowsList.mockResolvedValue([]);
  });

  it("loads tasks for the container on mount and renders the seven column headers with visible counts", async () => {
    m.ipc.tasksList.mockResolvedValue([
      makeTask({ id: "a", stage: "Plan" }),
      makeTask({ id: "b", stage: "Plan", state: needsYou({ type: "StageDone" }) }),
      makeTask({ id: "c", stage: "Done" }),
    ]);
    mountUi(<Board {...props} />);
    expect(await screen.findByRole("heading", { name: "Plan" })).toBeInTheDocument();
    const headings = screen.getAllByRole("heading", { level: 3 }).map((h) => h.textContent);
    expect(headings).toEqual(["Spec", "Plan", "Build", "Review", "Test", "Commit", "Done"]);
    expect(within(screen.getByRole("region", { name: "Plan" })).getByTestId("column-count")).toHaveTextContent("2");
    expect(m.ipc.tasksList).toHaveBeenCalledWith(projectContainer("p-1"));
    expect(m.ipc.on).toHaveBeenCalledWith("task-changed", expect.any(Function));
  });
  it("keeps the column row wider than 1,200 px so narrower windows scroll horizontally", async () => {
    m.ipc.tasksList.mockResolvedValue([makeTask({ id: "a" })]);
    mountUi(<Board {...props} />);
    expect(await screen.findByTestId("column-grid")).toHaveClass("min-w-[1200px]");
    expect(screen.getByTestId("column-scroller")).toHaveClass("overflow-x-auto");
  });
  it("toggling Needs you in the header filters the cards and the counts", async () => {
    m.ipc.tasksList.mockResolvedValue([
      makeTask({ id: "a", title: "Idle one", stage: "Plan" }),
      makeTask({ id: "b", title: "Waiting one", stage: "Plan", state: needsYou({ type: "StageDone" }) }),
    ]);
    mountUi(<Board {...props} />);
    await userEvent.click(await screen.findByRole("button", { name: "Needs you" }));
    expect(screen.queryByText("Idle one")).toBeNull();
    expect(screen.getByText("Waiting one")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Plan" })).getByTestId("column-count")).toHaveTextContent("1");
  });
  it("the filter button shows the active count and Clear resets it", async () => {
    m.ipc.tasksList.mockResolvedValue([makeTask({ id: "a" })]);
    mountUi(<Board {...props} />);
    await userEvent.click(await screen.findByRole("button", { name: "Needs you" }));
    await userEvent.click(screen.getByRole("button", { name: /Filter/ }));
    await userEvent.click(screen.getByRole("switch", { name: "Show hidden" }));
    expect(screen.getByRole("button", { name: /Filter/ })).toHaveTextContent("2");
    await userEvent.click(screen.getByRole("button", { name: "Clear" }));
    expect(screen.getByRole("button", { name: /Filter/ })).not.toHaveTextContent("2");
  });
  it("opens a task through the host with the click target", async () => {
    m.ipc.tasksList.mockResolvedValue([makeTask({ id: "a", title: "Open me", stage: "Plan" })]);
    const onOpenTask = vi.fn();
    mountUi(<Board {...props} onOpenTask={onOpenTask} />);
    await userEvent.click(await screen.findByText("Open me"));
    expect(onOpenTask).toHaveBeenCalledWith("a", { kind: "current" });
  });
  it("unsubscribes from events on unmount", async () => {
    m.ipc.tasksList.mockResolvedValue([makeTask({ id: "a" })]);
    const { unmount } = mountUi(<Board {...props} />);
    await screen.findByTestId("column-grid");
    unmount();
    expect(m.unlisten).toHaveBeenCalled();
  });
});
