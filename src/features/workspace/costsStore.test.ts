import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands, events } from "@/test/mockBindings";
import { costsStore } from "./costsStore";
import { summary } from "./costTestSupport";

const ok = <T>(data: T) => ({ status: "ok" as const, data });

describe("costsStore", () => {
  beforeEach(() => {
    costsStore.reset();
    vi.clearAllMocks();
  });
  afterEach(() => vi.useRealTimers());

  it("load requests the scope and reloads on cost-updated", async () => {
    commands.costSummary.mockResolvedValue(ok(summary({})));
    commands.settingsGet.mockResolvedValue(ok({ cost_overview: true }));
    await costsStore.load({ workspace: "ws-1" });
    expect(commands.costSummary).toHaveBeenCalledWith(expect.objectContaining({ workspace: "ws-1" }));
    expect(costsStore.get().group).toBe("project");
    vi.useFakeTimers();
    events.costUpdated.emitForTest({ task_id: "task-1" });
    await vi.advanceTimersByTimeAsync(1000);
    expect(commands.costSummary).toHaveBeenCalledTimes(2);
  });

  it("a project scope groups by task and reads cost_overview", async () => {
    commands.costSummary.mockResolvedValue(ok(summary({})));
    commands.settingsGet.mockResolvedValue(ok({ cost_overview: false }));
    await costsStore.load({ project: "proj-1" });
    expect(costsStore.get().group).toBe("task");
    expect(costsStore.get().enabled).toBe(false);
  });
});
