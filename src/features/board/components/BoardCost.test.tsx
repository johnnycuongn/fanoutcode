import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("../ipc");
vi.mock("@/components/ui/EmptyStateLottie", () => ({ default: () => null })); // jsdom has no WebAssembly
import { resetCostStore } from "../cost";
import { resetBoardStore } from "../store";
import { makeCost, makeProject, makeTask, projectContainer } from "../testing/fixtures";
import { mockIpc } from "../testing/mockIpc";
import { mountUi } from "../testing/render";
import { Board } from "./Board";

const base = {
  detailLevel: "Detailed",
  detailLevelByProject: {},
  workspaceBoard: "Swimlanes",
  projectLayer: "MultiRoot",
} as const;
const props = {
  container: projectContainer("p-1"),
  projects: [makeProject({ id: "p-1" })],
  focusedProject: null,
  currentWindowId: "w-1",
  onOpenTask: vi.fn(),
  onNewTask: vi.fn(),
};

describe("cost on the board", () => {
  let m: ReturnType<typeof mockIpc>;
  beforeEach(() => {
    resetBoardStore();
    resetCostStore();
    m = mockIpc();
    m.ipc.tasksList.mockResolvedValue([makeTask({ id: "a", title: "Costly" })]);
  });
  it("with the overview off no cost is requested or shown", async () => {
    mountUi(<Board {...props} settings={{ ...base, costOverview: false }} />);
    await screen.findByText("Costly");
    expect(m.ipc.costSummary).not.toHaveBeenCalled();
    expect(screen.queryByText("on board")).toBeNull();
  });
  it("with the overview on the header shows the board total and Detailed cards show their own cost", async () => {
    m.ipc.costSummary.mockImplementation(async (scope) =>
      scope.task ? makeCost({ cost_usd: 0.42, estimated: true }) : makeCost({ cost_usd: 3.5 }),
    );
    mountUi(<Board {...props} settings={{ ...base, costOverview: true }} />);
    expect(await screen.findByText("$3.50")).toBeInTheDocument();
    expect(screen.getByText("on board")).toBeInTheDocument();
    expect(await screen.findByText("~$0.42")).toBeInTheDocument();
  });
});
