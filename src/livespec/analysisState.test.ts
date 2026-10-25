import { describe, expect, it } from "vitest";
import type { LiveSpecEvent } from "@/lib/bindings";
import { applyEvent, initialAnalysis, startAnalysis } from "./analysisState";
import { checkSummary } from "./checks";

const run = (events: LiveSpecEvent[]) => events.reduce(applyEvent, startAnalysis(initialAnalysis, "draft"));
const result = { claim: "`x` is wrong", verdict: "confirmed", confidence: 0.9, path: "src/x.ts", line: 3 };
const check: LiveSpecEvent = { type: "Check", result };
const done = (text: string, partial = false): LiveSpecEvent => ({ type: "Done", text, partial });

describe("applyEvent", () => {
  it("streams text, then completes", () => {
    const s = run([
      { type: "Started" },
      { type: "Text", text: "## Intent\n", lane: 0 },
      { type: "Text", text: "- a", lane: 0 },
      done(""),
    ]);
    expect(s.status).toBe("done");
    expect(s.markdown).toBe("## Intent\n- a");
  });

  it("keeps applying checks after done", () => {
    const s = run([done("## Faults\n- `x` is wrong"), { type: "Checking", count: 1, total: 2 }, check]);
    expect(s.status).toBe("done");
    expect(s.checking).toEqual({ count: 1, total: 2 });
    expect(s.checks).toEqual([result]);
  });

  it("a notice never turns the notes into an error", () => {
    const s = run([done("## Intent\n- a"), { type: "Notice", message: "Couldn't check" }]);
    expect(s.status).toBe("done");
    expect(s.markdown).toBe("## Intent\n- a");
    expect(s.notices).toEqual(["Couldn't check"]);
  });

  it("keeps the old notes visible until the new run starts", () => {
    const before = run([
      done("## Intent\n- old"),
      { type: "Checking", count: 1, total: 1 },
      check,
      { type: "Notice", message: "x" },
    ]);
    const pending = startAnalysis(before, "new draft");
    expect(pending.status).toBe("thinking");
    expect(pending.markdown).toBe("## Intent\n- old");
    const started = applyEvent(pending, { type: "Started" });
    expect(started.markdown).toBe("");
    expect(started.checks).toEqual([]);
    expect(started.checking).toBeNull();
    expect(started.notices).toEqual([]);
    expect(started.checksStopped).toBe(false);
  });

  it("a skipped pause keeps the notes and shows them as current", () => {
    const before = run([done("## Intent\n- a")]);
    const s = applyEvent(startAnalysis(before, "draft, reworded"), { type: "Skipped" });
    expect(s.status).toBe("done");
    expect(s.markdown).toBe("## Intent\n- a");
    expect(s.analyzedDraft).toBe("draft, reworded");
  });

  it("an error keeps its message", () => {
    expect(run([{ type: "Error", message: "Install Claude Code first." }])).toMatchObject({
      status: "error",
      error: "Install Claude Code first.",
    });
  });
});

describe("checks interrupted by a skipped pause", () => {
  it("stop claiming to be checking", () => {
    const s = applyEvent(run([done("## Faults\n- `x`"), { type: "Checking", count: 4, total: 4 }, check]), {
      type: "Skipped",
    });
    expect(s.status).toBe("done");
    expect(s.notices).toEqual(["Checks stopped when you edited."]);
    expect(s.checksStopped).toBe(true);
    expect(checkSummary(s.checking!, s.checks.length, s.checksStopped)).toBe("1 of 4 conflicts checked");
  });

  it("finished checks are left alone", () => {
    const s = applyEvent(run([done(""), { type: "Checking", count: 1, total: 1 }, check]), { type: "Skipped" });
    expect(s.notices).toEqual([]);
    expect(s.checksStopped).toBe(false);
  });
});

describe("parallel lanes", () => {
  it("builds the notes from lanes in lane order", () => {
    const s = run([
      { type: "Started" },
      { type: "Text", text: "## Gaps\n- d", lane: 1 },
      { type: "Text", text: "## Intent\n- i", lane: 0 },
    ]);
    expect(s.markdown).toBe("## Intent\n- i\n## Gaps\n- d");
  });

  it("started clears the lane buffers too", () => {
    const s = run([{ type: "Started" }, { type: "Text", text: "## Intent\n- old", lane: 0 }, done("## Intent\n- old")]);
    const next = applyEvent(startAnalysis(s, "v2"), { type: "Started" });
    expect(applyEvent(next, { type: "Text", text: "## Faults\n- new", lane: 1 }).markdown).toBe("## Faults\n- new");
  });
});

describe("partly written notes", () => {
  it("offer a retry", () => {
    expect(run([{ type: "Started" }, done("## Intent\n- i", true)]).partial).toBe(true);
  });

  it("a fresh run clears the retry offer", () => {
    expect(run([done("x", true), { type: "Started" }]).partial).toBe(false);
  });
});
