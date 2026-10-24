import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("../ipc");
vi.mock("@/components/ui/EmptyStateLottie", () => ({ default: () => null })); // jsdom has no WebAssembly
import { STAGES } from "../columns";
import { resetBoardStore, setFilters } from "../store";
import { makeProject, makeTask, projectContainer } from "../testing/fixtures";
import { mockIpc } from "../testing/mockIpc";
import { mountUi } from "../testing/render";
import { Board } from "./Board";
import { COLUMN_HINTS, EmptyColumn } from "./EmptyColumn";

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

describe("EmptyColumn", () => {
  it.each(STAGES)("empty %s column shows its hint line with an illustration", (stage) => {
    render(<EmptyColumn stage={stage} compact={false} />);
    expect(screen.getByText(COLUMN_HINTS[stage])).toBeInTheDocument();
    expect(screen.getByTestId("empty-state")).toBeInTheDocument();
  });
  it("compact cells show the hint without the illustration", () => {
    render(<EmptyColumn stage="Build" compact />);
    expect(screen.getByText(COLUMN_HINTS.Build)).toBeInTheDocument();
    expect(screen.getByTestId("empty-hint")).toBeInTheDocument();
    expect(screen.queryByTestId("empty-state")).toBeNull();
  });
});

describe("BoardEmpty", () => {
  let m: ReturnType<typeof mockIpc>;
  beforeEach(() => {
    resetBoardStore();
    m = mockIpc();
    m.ipc.windowsList.mockResolvedValue([]);
  });
  it("an empty board offers New task with ⌘T", async () => {
    m.ipc.tasksList.mockResolvedValue([]);
    mountUi(<Board {...props} />);
    await userEvent.click(await screen.findByRole("button", { name: /New task/ }));
    expect(props.onNewTask).toHaveBeenCalled();
    expect(screen.queryByRole("heading", { name: "Spec" })).toBeNull();
    expect(screen.getByText("No tasks yet")).toBeInTheDocument();
  });
  it("a board emptied by filters offers Clear filters", async () => {
    m.ipc.tasksList.mockResolvedValue([makeTask({ id: "a", title: "Only card" })]);
    mountUi(<Board {...props} />);
    await screen.findByText("Only card");
    act(() => setFilters({ needsYou: true }));
    expect(await screen.findByText("No cards match these filters")).toBeInTheDocument();
    await userEvent.click(await screen.findByRole("button", { name: "Clear filters" }));
    expect(await screen.findByText("Only card")).toBeInTheDocument();
  });
  it("the columns show again once a task arrives", async () => {
    m.ipc.tasksList.mockResolvedValue([]);
    mountUi(<Board {...props} />);
    await screen.findByText("No tasks yet");
    act(() => m.emit("task-changed", { task: makeTask({ id: "new", title: "Arrived" }) }));
    expect(await screen.findByText("Arrived")).toBeInTheDocument();
    expect(screen.queryByText("No tasks yet")).toBeNull();
  });
  it("an empty column in a populated board shows its hint and can still take a drop", async () => {
    m.ipc.tasksList.mockResolvedValue([makeTask({ id: "a", stage: "Plan" })]);
    mountUi(<Board {...props} />);
    await screen.findByText(COLUMN_HINTS.Spec);
    expect(screen.getByRole("region", { name: "Spec" }).querySelector("[data-testid=column-body]")).not.toBeNull();
  });
});
