import { act, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("../ipc");
vi.mock("@/components/ui/EmptyStateLottie", () => ({ default: () => null })); // jsdom has no WebAssembly
import { SWATCHES, swatchClass } from "@/lib/theme";
import { reorderLanes, resetBoardStore } from "../store";
import { makeProject, makeTask, needsYou, workspaceContainer } from "../testing/fixtures";
import { mockIpc } from "../testing/mockIpc";
import { mountUi } from "../testing/render";
import { Board } from "./Board";

const settings = {
  detailLevel: "Simple",
  detailLevelByProject: { "p-2": "Medium" },
  costOverview: false,
  workspaceBoard: "Swimlanes",
  projectLayer: "MultiRoot",
} as const;
const projects = [
  makeProject({ id: "p-1", name: "Alpha", color: SWATCHES[0] }),
  makeProject({ id: "p-2", name: "Beta", color: SWATCHES[1] }),
];
const props = {
  container: workspaceContainer("ws-1"),
  projects,
  settings,
  focusedProject: null,
  currentWindowId: "w-1",
  onOpenTask: vi.fn(),
  onNewTask: vi.fn(),
};
const scrollSpy = vi.fn();
const LANES_KEY = "teamcode.board.lanes.Workspace.ws-1";

describe("Swimlanes", () => {
  let m: ReturnType<typeof mockIpc>;
  beforeEach(() => {
    resetBoardStore();
    localStorage.clear();
    m = mockIpc();
    m.ipc.windowsList.mockResolvedValue([]);
    Element.prototype.scrollIntoView = scrollSpy;
    scrollSpy.mockClear();
    m.ipc.tasksList.mockResolvedValue([
      makeTask({ id: "a", project_id: "p-1", title: "Alpha task", stage: "Build" }),
      makeTask({
        id: "b",
        project_id: "p-2",
        title: "Beta task",
        stage: "Build",
        state: needsYou({ type: "StageDone" }),
      }),
    ]);
  });

  it("renders one row per project with its colour, name and needs-you count", async () => {
    mountUi(<Board {...props} />);
    const rows = await screen.findAllByRole("row");
    expect(rows.map((r) => r.getAttribute("data-project"))).toEqual(["p-1", "p-2"]);
    expect(within(rows[1]!).getByText("Beta")).toBeInTheDocument();
    expect(within(rows[1]!).getByTestId("lane-needs-you")).toHaveTextContent("1");
    expect(within(rows[0]!).queryByTestId("lane-needs-you")).toBeNull();
    expect(within(rows[0]!).getByTestId("lane-color")).toHaveClass(swatchClass(SWATCHES[0], "bg"));
  });
  it("collapsing a row hides its columns and remembers it", async () => {
    mountUi(<Board {...props} />);
    await userEvent.click(await screen.findByRole("button", { name: "Collapse Alpha" }));
    expect(screen.queryByText("Alpha task")).toBeNull();
    expect(screen.getByText("Beta task")).toBeInTheDocument();
    expect(JSON.parse(localStorage.getItem(LANES_KEY)!).collapsed).toEqual(["p-1"]);
    await userEvent.click(screen.getByRole("button", { name: "Expand Alpha" }));
    expect(screen.getByText("Alpha task")).toBeInTheDocument();
  });
  it("starts with the rows collapsed that were saved as collapsed", async () => {
    localStorage.setItem(LANES_KEY, JSON.stringify({ order: ["p-1", "p-2"], collapsed: ["p-2"] }));
    mountUi(<Board {...props} />);
    await screen.findByText("Alpha task");
    expect(screen.queryByText("Beta task")).toBeNull();
  });
  it("rows render in the saved lane order and reorderLanes persists", async () => {
    mountUi(<Board {...props} />);
    await screen.findAllByRole("row");
    act(() => reorderLanes(1, 0));
    expect((await screen.findAllByRole("row")).map((r) => r.getAttribute("data-project"))).toEqual(["p-2", "p-1"]);
    expect(JSON.parse(localStorage.getItem(LANES_KEY)!).order).toEqual(["p-2", "p-1"]);
  });
  it("the focused project row is highlighted and scrolled into view", async () => {
    const { rerender } = mountUi(<Board {...props} />);
    await screen.findAllByRole("row");
    rerender(<Board {...props} focusedProject="p-2" />);
    expect(screen.getAllByRole("row")[1]).toHaveAttribute("data-focused", "true");
    expect(scrollSpy).toHaveBeenCalled();
  });
  it("cards in a lane use that project detail level override", async () => {
    mountUi(<Board {...props} />);
    const rows = await screen.findAllByRole("row");
    expect(within(rows[0]!).getByRole("article")).toHaveAttribute("data-level", "Simple");
    expect(within(rows[1]!).getByRole("article")).toHaveAttribute("data-level", "Medium");
  });
  it("empty swimlane cells show the hint without a Lottie", async () => {
    mountUi(<Board {...props} />);
    const rows = await screen.findAllByRole("row");
    expect(within(rows[0]!).queryByTestId("empty-state")).toBeNull();
    expect(within(rows[0]!).getAllByTestId("empty-hint")).toHaveLength(6);
  });
  it("a task-changed for a task in another project moves it between rows by project, never between lanes", async () => {
    mountUi(<Board {...props} />);
    const rows = await screen.findAllByRole("row");
    act(() =>
      m.emit("task-changed", { task: makeTask({ id: "a", project_id: "p-1", title: "Alpha task", stage: "Done" }) }),
    );
    expect(within(rows[0]!).getByRole("region", { name: "Done" })).toHaveTextContent("Alpha task");
  });
});
