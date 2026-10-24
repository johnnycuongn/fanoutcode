import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("../ipc");
vi.mock("@/components/ui/EmptyStateLottie", () => ({ default: () => null })); // jsdom has no WebAssembly
import { SWATCHES, swatchClass } from "@/lib/theme";
import { boardStore, resetBoardStore } from "../store";
import { makeProject, makeTask, needsYou, workspaceContainer } from "../testing/fixtures";
import { mockIpc } from "../testing/mockIpc";
import { mountUi } from "../testing/render";
import { Board } from "./Board";

const base = {
  detailLevel: "Simple",
  detailLevelByProject: {},
  costOverview: false,
  projectLayer: "MultiRoot",
} as const;
const projects = [
  makeProject({ id: "p-1", name: "Alpha", color: SWATCHES[0] }),
  makeProject({ id: "p-2", name: "Beta", color: SWATCHES[1] }),
];
const props = {
  container: workspaceContainer("ws-1"),
  projects,
  focusedProject: null,
  currentWindowId: "w-1",
  onOpenTask: vi.fn(),
  onNewTask: vi.fn(),
};
const oneProject = { ...base, workspaceBoard: "OneProject" } as const;

async function choose(name: string) {
  const user = userEvent.setup();
  screen.getByRole("combobox", { name: "Project" }).focus();
  await user.keyboard("{ArrowDown}");
  await user.click(await screen.findByRole("option", { name }));
}

describe("workspace board modes", () => {
  let m: ReturnType<typeof mockIpc>;
  beforeEach(() => {
    resetBoardStore();
    localStorage.clear();
    m = mockIpc();
    m.ipc.windowsList.mockResolvedValue([]);
    m.ipc.tasksList.mockResolvedValue([
      makeTask({ id: "a", project_id: "p-1", title: "Alpha task" }),
      makeTask({ id: "b", project_id: "p-2", title: "Beta task", state: needsYou({ type: "StageDone" }) }),
      makeTask({ id: "c", project_id: "p-2", title: "Beta two" }),
    ]);
  });

  it("one_project shows a picker; picking a project shows only its columns", async () => {
    mountUi(<Board {...props} settings={oneProject} />);
    await screen.findAllByRole("row");
    await choose("Beta");
    await waitFor(() => expect(screen.queryByText("Alpha task")).toBeNull());
    expect(screen.getByText("Beta task")).toBeInTheDocument();
    expect(screen.queryByRole("row")).toBeNull();
    expect(screen.getByRole("combobox", { name: "Project" })).toHaveTextContent("Beta");
  });
  it("one_project with All shows swimlanes read-only", async () => {
    mountUi(<Board {...props} settings={oneProject} />);
    const rows = await screen.findAllByRole("row");
    expect(rows).toHaveLength(2);
    expect(within(rows[0]!).getByRole("article")).toHaveAttribute("aria-disabled", "true");
    expect(screen.queryByRole("button", { name: "Reorder Alpha" })).toBeNull();
    expect(screen.getByRole("combobox", { name: "Project" })).toHaveTextContent("All");
  });
  it("focusedProject from the host drives the picker and clearing it returns to All", async () => {
    const { rerender } = mountUi(<Board {...props} settings={oneProject} focusedProject="p-1" />);
    await screen.findByText("Alpha task");
    expect(screen.getByRole("combobox", { name: "Project" })).toHaveTextContent("Alpha");
    rerender(<Board {...props} settings={oneProject} focusedProject={null} />);
    await waitFor(() => expect(screen.getByRole("combobox", { name: "Project" })).toHaveTextContent("All"));
  });
  it("a project picked by hand survives a re-render with the same focus", async () => {
    const { rerender } = mountUi(<Board {...props} settings={oneProject} />);
    await screen.findAllByRole("row");
    await choose("Beta");
    rerender(<Board {...props} settings={{ ...oneProject }} />);
    expect(screen.getByRole("combobox", { name: "Project" })).toHaveTextContent("Beta");
  });
  it("rail layer forces one_project regardless of workspace_board", async () => {
    mountUi(
      <Board {...props} settings={{ ...base, workspaceBoard: "Mixed", projectLayer: "Rail" }} focusedProject="p-2" />,
    );
    expect(await screen.findByText("Beta task")).toBeInTheDocument();
    expect(screen.queryByText("Alpha task")).toBeNull();
    expect(boardStore.get().pickedProject).toBe("p-2");
  });
  it("mixed shows one set of columns with a project badge on every card", async () => {
    mountUi(<Board {...props} settings={{ ...base, workspaceBoard: "Mixed" }} />);
    const cards = await screen.findAllByRole("article");
    expect(cards).toHaveLength(3);
    expect(screen.queryByRole("row")).toBeNull();
    const beta = cards.find((c) => c.textContent!.includes("Beta task"))!;
    expect(within(beta).getByText("Beta")).toHaveClass(swatchClass(SWATCHES[1], "bg"));
  });
  it("mixed chip row shows each project with its count and a Needs you chip, and chips toggle filters", async () => {
    mountUi(<Board {...props} settings={{ ...base, workspaceBoard: "Mixed" }} />);
    const row = await screen.findByRole("toolbar", { name: "Projects" });
    expect(within(row).getByRole("button", { name: /Alpha/ })).toHaveTextContent("1");
    expect(within(row).getByRole("button", { name: /Beta/ })).toHaveTextContent("2");
    expect(within(row).getByRole("button", { name: /Needs you/ })).toHaveTextContent("1");
    await userEvent.click(within(row).getByRole("button", { name: /Alpha/ }));
    expect(screen.queryByText("Beta task")).toBeNull();
    expect(screen.getByText("Alpha task")).toBeInTheDocument();
    expect(within(row).getByRole("button", { name: /Beta/ })).toHaveTextContent("2");
    await userEvent.click(within(row).getByRole("button", { name: /Alpha/ }));
    await userEvent.click(within(row).getByRole("button", { name: /Needs you/ }));
    expect(screen.getAllByRole("article")).toHaveLength(1);
  });
  it("mixed keeps the chip row when the chosen project has no cards, so the filter can be undone", async () => {
    m.ipc.tasksList.mockResolvedValue([makeTask({ id: "a", project_id: "p-1", title: "Alpha task" })]);
    mountUi(<Board {...props} settings={{ ...base, workspaceBoard: "Mixed" }} />);
    const row = await screen.findByRole("toolbar", { name: "Projects" });
    await userEvent.click(within(row).getByRole("button", { name: /Beta/ }));
    expect(await screen.findByText("No cards match these filters")).toBeInTheDocument();
    await userEvent.click(
      within(screen.getByRole("toolbar", { name: "Projects" })).getByRole("button", { name: /Beta/ }),
    );
    expect(await screen.findByText("Alpha task")).toBeInTheDocument();
  });
  it("the project chip of the host's focused project is marked", async () => {
    mountUi(<Board {...props} settings={{ ...base, workspaceBoard: "Mixed" }} focusedProject="p-2" />);
    const row = await screen.findByRole("toolbar", { name: "Projects" });
    await act(async () => {});
    expect(within(row).getByRole("button", { name: /Beta/ })).toHaveAttribute("data-focused", "true");
    expect(within(row).getByRole("button", { name: /Alpha/ })).toHaveAttribute("data-focused", "false");
  });
});
