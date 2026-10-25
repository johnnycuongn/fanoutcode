import { describe, it, expect, vi } from "vitest";
import { screen } from "@testing-library/react";
import { renderUi } from "@/test/ui";
import { TASK_ID } from "../fixtures";

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
    settings: { detail: "Simple", showCost: false },
  };
  return { useCanvas: () => state };
});
const { Outline } = await import("./Outline");

describe("Outline", () => {
  it("renders the checklist with done boxes checked and streams under active steps only", () => {
    renderUi(<Outline taskId={TASK_ID} />);
    const boxes = screen.getAllByRole("checkbox");
    expect(boxes).toHaveLength(6);
    expect(boxes[0].getAttribute("aria-checked")).toBe("true");
    expect(boxes[1].getAttribute("aria-checked")).toBe("false");
    // steps 2 and 4 are both a3's, so its edit shows under each
    expect(screen.getAllByText("Edit /repo/app/src/toolbar.tsx")).toHaveLength(2);
    expect(screen.queryByText("Bash npm test")).toBeNull(); // step 3 is done, collapsed
  });
});
