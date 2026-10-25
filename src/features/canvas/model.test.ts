import { describe, it, expect } from "vitest";
import type { TaskEvent } from "@/lib/bindings";
import { deriveCanvas } from "./model";
import { fx, sampleBuild, samplePlan, ROOTS, MEMORY_DIR } from "./fixtures";

describe("deriveCanvas", () => {
  it("composes agents, steps, files and edits from the sample build", () => {
    const m = deriveCanvas({ steps: samplePlan(), events: sampleBuild(), memoryDir: MEMORY_DIR, roots: ROOTS });
    expect(m.agents.map((a) => a.agentId)).toEqual(["main", "a1", "a2", "a3"]);
    expect(m.files.map((f) => [f.path, f.repo])).toContainEqual(["/repo/api/src/route.ts", "/repo/api"]);
    expect(m.steps[2].repo).toBe("/repo/api");
    expect(m.steps[0].repo).toBe("/repo/app");
    expect(m.agents[1].files).toEqual(["file:/repo/app/src/a.ts"]);
    expect(m.lastSeq).toBe(18);
  });
  it("carries agent costs onto agent nodes and leaves unknown agents null", () => {
    const m = deriveCanvas({
      steps: samplePlan(),
      events: sampleBuild(),
      memoryDir: MEMORY_DIR,
      roots: ROOTS,
      costs: [{ agentId: "a1", costUsd: 0.12 }],
    });
    expect(m.agents.find((a) => a.agentId === "a1")?.costUsd).toBe(0.12);
    expect(m.agents.find((a) => a.agentId === "a2")?.costUsd).toBeNull();
  });
  it("is empty but valid with no plan and no events", () => {
    const m = deriveCanvas({ steps: [], events: [], memoryDir: MEMORY_DIR, roots: [] });
    expect(m.agents).toHaveLength(1);
    expect(m.steps).toEqual([]);
    expect(m.files).toEqual([]);
    expect(m.lastSeq).toBe(0);
  });
  it("derives a 20,000-event stream in under one second", () => {
    const events: TaskEvent[] = [];
    let seq = 1;
    for (let i = 0; i < 2000; i++) {
      events.push(fx.agentStart(seq++, `a${i}`, "frontend", `Step ${1 + (i % 6)} · frontend: part ${i}`));
      for (let j = 0; j < 8; j++)
        events.push(fx.edit(seq++, `a${i}`, `/repo/app/src/f${(i * 8 + j) % 500}.ts`, "a", "b"));
      events.push(fx.agentStop(seq++, `a${i}`));
    }
    const t0 = performance.now();
    deriveCanvas({ steps: samplePlan(), events, memoryDir: MEMORY_DIR, roots: ROOTS });
    expect(performance.now() - t0).toBeLessThan(1000);
  });
});
