import { describe, it, expect, vi } from "vitest";
import { screen, fireEvent } from "@testing-library/react";
import { renderUi } from "@/test/ui";
import { TASK_ID } from "../fixtures";

const { select } = vi.hoisted(() => ({ select: vi.fn() }));
vi.mock("../store", async () => {
  const fx = await import("../fixtures");
  const events = fx.sampleBuild();
  const model = (await import("../model")).deriveCanvas({
    steps: fx.samplePlan(),
    events,
    memoryDir: fx.MEMORY_DIR,
    roots: fx.ROOTS,
  });
  const state = {
    taskId: fx.TASK_ID,
    task: { workspace_id: null },
    roots: fx.ROOTS,
    model,
    events,
    replaySeq: null,
    selection: { kind: "agent", id: "a2" },
    settings: { lens: "EventStream", detail: "Simple", showCost: false },
  };
  return { useCanvas: () => state, select };
});
const { DrillIn } = await import("./DrillIn");

describe("DrillIn", () => {
  it("shows the event stream for the selected agent and filters with the kind chips", () => {
    renderUi(<DrillIn taskId={TASK_ID} />);
    expect(screen.getByText("backend")).toBeTruthy();
    expect(screen.getAllByRole("row")).toHaveLength(5);
    fireEvent.click(screen.getByRole("button", { name: "command" }));
    expect(screen.getAllByRole("row")).toHaveLength(1);
    expect(screen.getByText("npm test")).toBeTruthy();
  });
  it("closes through select(null)", () => {
    renderUi(<DrillIn taskId={TASK_ID} />);
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(select).toHaveBeenCalledWith(TASK_ID, null);
  });
});
