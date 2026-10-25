import { describe, it, expect } from "vitest";
import { eventsUntil, replayBounds, seqAtTime, canReplay } from "./replay";
import { deriveCanvas } from "./model";
import { sampleBuild, samplePlan, ROOTS, MEMORY_DIR } from "./fixtures";

describe("replay", () => {
  const events = sampleBuild();
  it("reports bounds in seq and time", () => {
    expect(replayBounds(events)).toEqual({
      firstSeq: 1,
      lastSeq: 18,
      startAt: "2026-10-24T05:00:01.000Z",
      endAt: "2026-10-24T05:00:18.000Z",
    });
    expect(replayBounds([])).toBeNull();
  });
  it("maps a time to the latest seq at or before it", () => {
    expect(seqAtTime(events, Date.parse("2026-10-24T05:00:09.500Z"))).toBe(9);
    expect(seqAtTime(events, Date.parse("2026-10-24T04:00:00Z"))).toBe(1);
  });
  it("shows a file as unshared before the second edit and shared after it", () => {
    const at = (seq: number) =>
      deriveCanvas({
        steps: samplePlan(),
        events: eventsUntil(events, seq),
        memoryDir: MEMORY_DIR,
        roots: ROOTS,
      }).files.find((f) => f.path === "/repo/app/src/a.ts")!;
    expect(at(8).sharedAt).toBeNull();
    expect(at(9).sharedAt).toBe(9);
  });
  it("allows replay only when the agent is not working and events exist", () => {
    expect(canReplay({ state: { type: "Idle" } }, events)).toBe(true);
    expect(canReplay({ state: { type: "NeedsYou", reason: { type: "StageDone" } } }, events)).toBe(true);
    expect(canReplay({ state: { type: "Working" } }, events)).toBe(false);
    expect(canReplay({ state: { type: "Idle" } }, [])).toBe(false);
  });
});
