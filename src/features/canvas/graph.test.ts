import { describe, it, expect } from "vitest";
import { deriveCanvas } from "./model";
import { toGraph, layoutKey, nodeId } from "./graph";
import { repoFor, showRepoBadges } from "./repo";
import { sampleBuild, samplePlan, ROOTS, MEMORY_DIR } from "./fixtures";

const model = () => deriveCanvas({ steps: samplePlan(), events: sampleBuild(), memoryDir: MEMORY_DIR, roots: ROOTS });

describe("hub graph", () => {
  const g = toGraph(model(), "HubGraph");
  it("has agent, step and file nodes and only touches edges", () => {
    expect(g.nodes.filter((n) => n.kind === "agent")).toHaveLength(4);
    expect(g.nodes.filter((n) => n.kind === "step")).toHaveLength(6);
    expect(g.nodes.filter((n) => n.kind === "file")).toHaveLength(3);
    expect(g.edges.every((e) => e.kind === "touches")).toBe(true);
    expect(g.edges).toHaveLength(8);
  });
  it("draws two agent edges into a shared file", () => {
    const shared = nodeId.file("/repo/app/src/a.ts");
    expect(
      g.edges
        .filter((e) => e.target === shared)
        .map((e) => e.source)
        .sort(),
    ).toEqual(["agent:a1", "agent:a2"]);
  });
  it("marks edges from a working agent", () => {
    expect(g.edges.filter((e) => e.working).map((e) => e.source)).toEqual(["agent:a3", "agent:a3", "agent:a3"]);
  });
});

describe("flow canvas", () => {
  const g = toGraph(model(), "Flow");
  it("has no file nodes and keeps files inside step cards", () => {
    expect(g.nodes.some((n) => n.kind === "file")).toBe(false);
    const step1 = g.nodes.find((n) => n.id === "step:1")!;
    expect(step1.kind === "step" && step1.data.files.map((f) => f.path)).toEqual(["/repo/app/src/a.ts"]);
  });
  it("draws depends edges from (after …) and an implicit edge from the previous step", () => {
    const deps = g.edges
      .filter((e) => e.kind === "depends")
      .map((e) => e.id)
      .sort();
    expect(deps).toEqual([
      "step:1->step:2",
      "step:1->step:3",
      "step:2->step:4",
      "step:3->step:4",
      "step:4->step:5",
      "step:5->step:6",
    ]);
  });
  it("points only working agents at their current cards", () => {
    expect(g.edges.filter((e) => e.kind === "touches").map((e) => e.id)).toEqual([
      "agent:a3->step:2",
      "agent:a3->step:4",
    ]);
  });
});

describe("layoutKey", () => {
  it("changes when a step status changes and not when only a tool line changes", () => {
    const base = model();
    const key = layoutKey(toGraph(base, "HubGraph"));
    const retitled = { ...base, steps: base.steps.map((s) => ({ ...s, toolLine: "Read /x" })) };
    expect(layoutKey(toGraph(retitled, "HubGraph"))).toBe(key);
    const done = { ...base, steps: base.steps.map((s, i) => (i === 1 ? { ...s, status: "done" as const } : s)) };
    expect(layoutKey(toGraph(done, "HubGraph"))).not.toBe(key);
  });
});

describe("repo", () => {
  it("picks the longest matching root on a path boundary", () => {
    const roots = [...ROOTS, { root: "/repo/app/packages/ui", name: "ui", color: "violet" }];
    expect(repoFor("/repo/app/packages/ui/x.ts", roots)?.name).toBe("ui");
    expect(repoFor("/repo/application/x.ts", roots)).toBeNull();
  });
  it("shows badges only for workspace tasks with two or more roots", () => {
    expect(showRepoBadges({ workspace_id: "ws-1" }, ROOTS)).toBe(true);
    expect(showRepoBadges({ workspace_id: null }, ROOTS)).toBe(false);
    expect(showRepoBadges({ workspace_id: "ws-1" }, [ROOTS[0]])).toBe(false);
  });
});
