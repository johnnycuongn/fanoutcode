import { describe, it, expect } from "vitest";
import { teamShortcut, teamSummary, degradationNotice, DOCK_GROWN_FRACTION } from "./placement";
import { deriveCanvas } from "./model";
import { fx, sampleBuild, samplePlan, ROOTS, MEMORY_DIR } from "./fixtures";

const idle = { grown: false, shown: "terminal" as const, paneOpen: false };

describe("teamShortcut", () => {
  it("dock grows: grows to two thirds then restores", () => {
    const a = teamShortcut("DockGrows", idle);
    expect(a).toEqual({
      next: { ...idle, grown: true },
      action: { kind: "dock-fraction", fraction: DOCK_GROWN_FRACTION },
    });
    expect(teamShortcut("DockGrows", a.next).action).toEqual({ kind: "dock-fraction", fraction: "restore" });
    expect(DOCK_GROWN_FRACTION).toBeCloseTo(2 / 3);
  });
  it("swap: flips what the pane shows", () => {
    expect(teamShortcut("Swap", idle)).toEqual({
      next: { ...idle, shown: "team" },
      action: { kind: "show", which: "team" },
    });
    expect(teamShortcut("Swap", { ...idle, shown: "team" }).action).toEqual({ kind: "show", which: "terminal" });
  });
  it("own pane: opens the pane once, then focuses it", () => {
    const a = teamShortcut("OwnPane", idle);
    expect(a).toEqual({ next: { ...idle, paneOpen: true }, action: { kind: "open-pane" } });
    expect(teamShortcut("OwnPane", a.next).action).toEqual({ kind: "focus-pane" });
  });
});

describe("teamSummary", () => {
  it("reads working agents, the lowest active step and the file count", () => {
    const m = deriveCanvas({ steps: samplePlan(), events: sampleBuild(), memoryDir: MEMORY_DIR, roots: ROOTS });
    expect(teamSummary(m)).toBe("2 agents working · step 2/6 active · 3 files");
    expect(teamSummary(deriveCanvas({ steps: [], events: [], memoryDir: MEMORY_DIR, roots: [] }))).toBe(
      "No team activity yet",
    );
  });
});

describe("degradationNotice", () => {
  it("names unknown events and missing SubagentStart, and is null otherwise", () => {
    expect(degradationNotice(sampleBuild())).toBeNull();
    expect(degradationNotice([fx.unknown(1, "Fancy")])).toBe(
      "Claude Code sent events this version does not understand: Fancy. The team view may be incomplete.",
    );
    expect(
      degradationNotice([
        fx.pre(1, "main", "Task", { description: "Step 1 · frontend: x" }),
        fx.post(2, "main", "Task", {}),
      ]),
    ).toBe("No SubagentStart events arrived; agents are drawn from Task dispatches only.");
  });
});
