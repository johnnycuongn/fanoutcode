import { describe, it, expect } from "vitest";
import { miniDiff, byFileLens, eventStreamLens, streamAgents } from "./lens";
import { deriveCanvas } from "./model";
import { sampleBuild, samplePlan, ROOTS, MEMORY_DIR } from "./fixtures";

const events = sampleBuild();
const model = deriveCanvas({ steps: samplePlan(), events, memoryDir: MEMORY_DIR, roots: ROOTS });
const none = { kinds: new Set<never>(), agents: new Set<never>() };

describe("miniDiff", () => {
  it("lists removed then added lines and caps each side", () => {
    const d = miniDiff("a\nb", "c");
    expect(d.lines).toEqual([
      { sign: "-", text: "a" },
      { sign: "-", text: "b" },
      { sign: "+", text: "c" },
    ]);
    expect(miniDiff(Array(20).fill("x").join("\n"), "", 12)).toMatchObject({ truncated: true });
    expect(miniDiff(Array(20).fill("x").join("\n"), "", 12).lines).toHaveLength(12);
  });
});

describe("byFileLens", () => {
  it("for an agent lists its files, every owner, and only its edits with the step served", () => {
    const { groups, rollUp } = byFileLens(model, events, { kind: "agent", id: "a2" });
    expect(groups.map((g) => g.path)).toEqual(["/repo/api/src/route.ts", "/repo/app/src/a.ts"]);
    const a = groups[1];
    expect(a.owners).toEqual(["a1", "a2"]);
    expect(a.edits.map((e) => [e.seq, e.agentId, e.step])).toEqual([[9, "a2", 3]]);
    expect(rollUp.commands.map((c) => c.target)).toEqual(["npm test"]);
    expect(rollUp.hooks.map((h) => h.seq)).toEqual([7, 14]);
  });
  it("for a file lists every owner's edits in order", () => {
    const { groups } = byFileLens(model, events, { kind: "file", id: "file:/repo/app/src/a.ts" });
    expect(groups).toHaveLength(1);
    expect(groups[0].edits.map((e) => e.seq)).toEqual([5, 9]);
    expect(groups[0].edits[0].diff.lines).toEqual([
      { sign: "+", text: "line" },
      { sign: "+", text: "line" },
    ]);
  });
});

describe("eventStreamLens", () => {
  it("is newest first, pairs tool calls with duration and result, and mutes hook rows", () => {
    const rows = eventStreamLens(model, events, { kind: "agent", id: "a2" }, none);
    expect(rows.map((r) => r.seq)).toEqual([14, 12, 9, 8, 7]);
    expect(rows[1]).toMatchObject({
      kind: "command",
      tool: "Bash",
      target: "npm test",
      result: "ok",
      durationMs: 1000,
      muted: false,
    });
    expect(rows[0]).toMatchObject({ kind: "hook", tool: null, muted: true });
  });
  it("filters by kind and by agent", () => {
    const file = { kind: "file" as const, id: "file:/repo/app/src/a.ts" };
    const edits = eventStreamLens(model, events, file, { kinds: new Set(["edit"]), agents: new Set() });
    expect(edits.map((r) => [r.seq, r.agentId])).toEqual([
      [9, "a2"],
      [5, "a1"],
    ]);
    const onlyA1 = eventStreamLens(model, events, file, { kinds: new Set(), agents: new Set(["a1"]) });
    // stop, edit, start
    expect(onlyA1.map((r) => [r.seq, r.agentId])).toEqual([
      [10, "a1"],
      [5, "a1"],
      [4, "a1"],
    ]);
  });
  it("both lenses include hook firings", () => {
    expect(byFileLens(model, events, { kind: "agent", id: "a1" }).rollUp.hooks.length).toBeGreaterThan(0);
    expect(eventStreamLens(model, events, { kind: "agent", id: "a1" }, none).some((r) => r.kind === "hook")).toBe(true);
  });
  it("offers the target's agents as filter chips", () => {
    expect(streamAgents(model, { kind: "agent", id: "agent:a2" })).toEqual(["a2"]);
    expect(streamAgents(model, { kind: "file", id: "file:/repo/app/src/a.ts" })).toEqual(["a1", "a2"]);
  });
});
