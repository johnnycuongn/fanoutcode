import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("./ipc");
import {
  costStore,
  resetCostStore,
  formatCost,
  formatTokens,
  loadBoardCost,
  scopeFor,
  scopeForTask,
  startCostEvents,
  trackTaskCost,
  untrackTaskCost,
} from "./cost";
import { makeCost, projectContainer, workspaceContainer } from "./testing/fixtures";
import { mockIpc } from "./testing/mockIpc";

const sum = (cost_usd: number, estimated = false) =>
  makeCost({ cost_usd, input_tokens: 1000, output_tokens: 200, estimated });

describe("formatting", () => {
  it("formatCost prefixes ~ for estimates and shows two decimals", () => {
    expect(formatCost(sum(0.4219, true))).toBe("~$0.42");
    expect(formatCost(sum(12))).toBe("$12.00");
  });
  it("formatTokens abbreviates thousands and millions", () => {
    expect(formatTokens(999)).toBe("999");
    expect(formatTokens(12_300)).toBe("12.3k");
    expect(formatTokens(3_400_000)).toBe("3.4M");
    expect(formatTokens(2_000)).toBe("2k");
  });
});

describe("scopes", () => {
  it("scopeFor names the workspace or the project and nothing else", () => {
    expect(scopeFor(workspaceContainer("ws-1"))).toEqual({ workspace: "ws-1", project: null, task: null });
    expect(scopeFor(projectContainer("p-1"))).toEqual({ workspace: null, project: "p-1", task: null });
    expect(scopeForTask("t-1")).toEqual({ workspace: null, project: null, task: "t-1" });
  });
});

describe("costStore", () => {
  let m: ReturnType<typeof mockIpc>;
  beforeEach(() => {
    resetCostStore();
    m = mockIpc();
    m.ipc.costSummary.mockResolvedValue(sum(1));
  });
  it("loadBoardCost asks cost_summary for the container scope", async () => {
    await loadBoardCost(workspaceContainer("ws-1"));
    expect(m.ipc.costSummary).toHaveBeenCalledWith(scopeFor(workspaceContainer("ws-1")));
    expect(costStore.get().board).toEqual(sum(1));
  });
  it("trackTaskCost fetches the task once and untrack forgets it", async () => {
    trackTaskCost("t-1");
    trackTaskCost("t-1");
    await Promise.resolve();
    expect(m.ipc.costSummary).toHaveBeenCalledTimes(1);
    expect(m.ipc.costSummary).toHaveBeenCalledWith(scopeForTask("t-1"));
    await vi.waitFor(() => expect(costStore.get().byTask["t-1"]).toEqual(sum(1)));
    untrackTaskCost("t-1");
    expect(costStore.get().byTask["t-1"]).toBeUndefined();
  });
  it("cost-updated refetches only tracked tasks, plus the board total", async () => {
    await loadBoardCost(workspaceContainer("ws-1"));
    startCostEvents();
    trackTaskCost("t-1");
    await Promise.resolve();
    m.ipc.costSummary.mockClear();
    m.emit("cost-updated", { task_id: "t-2" });
    m.emit("cost-updated", { task_id: "t-1" });
    await Promise.resolve();
    const scopes = m.ipc.costSummary.mock.calls.map((c) => c[0]);
    expect(scopes.filter((s) => s.task === "t-1")).toHaveLength(1);
    expect(scopes.filter((s) => s.task === "t-2")).toHaveLength(0);
    expect(scopes.filter((s) => s.task === null)).toHaveLength(2);
  });
  it("cost-updated before any board load does not call cost_summary for the board", () => {
    startCostEvents();
    m.emit("cost-updated", { task_id: "t-2" });
    expect(m.ipc.costSummary).not.toHaveBeenCalled();
  });
  it("a failing cost_summary keeps the last numbers", async () => {
    await loadBoardCost(projectContainer("p-1"));
    m.ipc.costSummary.mockRejectedValue(new Error("no ledger"));
    await loadBoardCost(projectContainer("p-1"));
    expect(costStore.get().board).toEqual(sum(1));
  });
});
