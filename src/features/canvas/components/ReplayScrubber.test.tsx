import { describe, it, expect, vi } from "vitest";
import { screen, fireEvent } from "@testing-library/react";
import { renderUi } from "@/test/ui";
import { TASK_ID, sampleBuild } from "../fixtures";

process.env.TZ = "UTC"; // the label is local time; pin it for the assertion
const { setReplaySeq, stopReplay, view } = vi.hoisted(() => ({
  setReplaySeq: vi.fn(),
  stopReplay: vi.fn(),
  view: { replaySeq: 9 as number | null },
}));
vi.mock("../store", async () => {
  const fx = await import("../fixtures");
  const events = fx.sampleBuild();
  return {
    useCanvas: () => ({ taskId: fx.TASK_ID, events, replaySeq: view.replaySeq }),
    setReplaySeq,
    stopReplay,
  };
});
const { ReplayScrubber } = await import("./ReplayScrubber");

describe("ReplayScrubber", () => {
  it("moves the cursor with the slider and returns to live", () => {
    void sampleBuild;
    renderUi(<ReplayScrubber taskId={TASK_ID} />);
    const thumb = screen.getByRole("slider", { name: "Replay position" });
    expect(thumb.getAttribute("aria-valuetext")).toBe("05:00:09");
    expect(screen.getByText("9 of 18 events")).toBeTruthy();
    fireEvent.keyDown(thumb, { key: "ArrowRight" });
    expect(setReplaySeq).toHaveBeenCalledWith(TASK_ID, 10);
    fireEvent.click(screen.getByRole("button", { name: "Live" }));
    expect(stopReplay).toHaveBeenCalledWith(TASK_ID);
  });
  it("renders nothing when not replaying", () => {
    view.replaySeq = null;
    const { container } = renderUi(<ReplayScrubber taskId={TASK_ID} />);
    expect(container.querySelector("[data-replay]")).toBeNull();
  });
});
