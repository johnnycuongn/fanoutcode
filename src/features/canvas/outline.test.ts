import { describe, it, expect } from "vitest";
import { buildOutline, hookText } from "./outline";
import { deriveCanvas } from "./model";
import { fx, sampleBuild, samplePlan, ROOTS, MEMORY_DIR } from "./fixtures";

const events = sampleBuild();
const items = buildOutline(deriveCanvas({ steps: samplePlan(), events, memoryDir: MEMORY_DIR, roots: ROOTS }), events);

describe("buildOutline", () => {
  it("expands only active steps", () => {
    expect(items.map((i) => i.expanded)).toEqual([false, true, false, true, false, false]);
  });
  it("attaches only that step's agent events, in order, with hooks muted", () => {
    const step2 = items[1];
    expect(step2.agents).toEqual(["a3"]);
    expect(step2.stream.map((e) => [e.seq, e.kind, e.muted])).toEqual([
      [17, "hook", true],
      [18, "tool", false],
    ]);
    expect(step2.stream[0].text).toBe("started: Step 2, 4 · ui-polish: toolbar and wiring");
    expect(step2.stream[1].text).toBe("Edit /repo/app/src/toolbar.tsx");
  });
  it("carries an empty stream for collapsed steps even when they had work", () => {
    expect(items[0].stream).toEqual([]);
    expect(items[0].agents).toEqual(["a1"]); // a2's edit at seq 9 served step 3, not step 1
  });
});

describe("hookText", () => {
  it("words each hook the outline and the lenses show", () => {
    expect(hookText(fx.agentStop(1, "a1"))).toBe("stopped");
    expect(hookText(fx.permission(2, "Bash"))).toBe("permission: Bash");
    expect(hookText(fx.notification(3, "Needs input"))).toBe("Needs input");
    expect(hookText(fx.stop(4, "All done\nmore"))).toBe("All done");
    expect(hookText(fx.stopFailure(5, "rate limited"))).toBe("error: rate limited");
    expect(hookText(fx.claimDenied(6, { path: "src/a.ts" }, "t-b2"))).toBe("edit held: src/a.ts · t-b2");
    expect(hookText(fx.claimNoted(8, "src/a.ts", "t-b2"))).toBe("edited over: src/a.ts · t-b2");
    expect(hookText(fx.commandRan(9, "git add -A", "t-b2"))).toBe("command ran: git add -A · t-b2");
    expect(hookText(fx.overlapDecided(7, "src/a.ts", null, "Told"))).toBe("decided Told: src/a.ts");
  });
});
