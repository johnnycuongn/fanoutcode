import { describe, it, expect } from "vitest";
import type { CostSummary, PlanDoc, Project, RepoChanges, Task } from "@/lib/bindings";
import { fromRepoChanges, fromCostSummary, fromPlanDoc, rootsFor, scopeFor } from "./adapters";
import { ROOTS } from "./fixtures";

const file = (path: string, added: number, removed: number, owners: string[]) => ({
  path,
  status: "Modified" as const,
  added,
  removed,
  binary: false,
  owners,
  shared: owners.length > 1,
  source: "Hook" as const,
});

describe("adapters", () => {
  it("flattens attributed files to absolute paths with owners", () => {
    const changes: RepoChanges[] = [
      {
        repo_root: "/repo/app",
        base: null,
        files: [file("src/a.ts", 3, 1, ["a1"])],
        other: [file("junk.log", 1, 0, [])],
      },
    ];
    expect(fromRepoChanges(changes)).toEqual([{ path: "/repo/app/src/a.ts", added: 3, removed: 1, owners: ["a1"] }]);
  });
  it("reads per-agent cost from CostSummary.by_agent", () => {
    const summary = {
      by_agent: [
        { key: "a1", cost_usd: 0.12 },
        { key: "a2", cost_usd: 0.3 },
      ],
    } as unknown as CostSummary;
    expect(fromCostSummary(summary)).toEqual([
      { agentId: "a1", costUsd: 0.12 },
      { agentId: "a2", costUsd: 0.3 },
    ]);
  });
  it("puts the home root first and names linked roots from projects", () => {
    const task = { project_id: "p-a", linked_roots: ["/repo/api", "/elsewhere/lib"] } as unknown as Task;
    const projects = [
      { id: "p-a", name: "App", root: "/repo/app", color: "blue" },
      { id: "p-b", name: "Api", root: "/repo/api", color: "teal" },
    ] as unknown as Project[];
    expect(rootsFor(task, projects)).toEqual([
      { root: "/repo/app", name: "App", color: "blue" },
      { root: "/repo/api", name: "Api", color: "teal" },
      { root: "/elsewhere/lib", name: "lib", color: "mute" },
    ]);
  });
  it("scopes settings and cost to the task's workspace, project and id", () => {
    const task = { id: "t-1", project_id: "p-a", workspace_id: "ws-1" } as unknown as Task;
    expect(scopeFor(task)).toEqual({ workspace: "ws-1", project: "p-a", task: "t-1" });
  });
  it("reads plan steps: explicit deps only, owners from the Team section, repo heading to its root", () => {
    const doc: PlanDoc = {
      steps: [
        { number: 1, title: "Scaffold", done: true, deps: [], explicit_deps: false, repo: null },
        { number: 2, title: "Toolbar", done: false, deps: [1], explicit_deps: false, repo: null },
        { number: 3, title: "API route", done: false, deps: [1], explicit_deps: true, repo: "api" },
      ],
      team: [
        { role: "frontend", steps: [1, 2] },
        { role: "backend", steps: [3] },
      ],
    };
    expect(fromPlanDoc(doc, ROOTS)).toEqual([
      { number: 1, title: "Scaffold", done: true, after: [], owners: ["frontend"], files: [], repo: null },
      { number: 2, title: "Toolbar", done: false, after: [], owners: ["frontend"], files: [], repo: null },
      { number: 3, title: "API route", done: false, after: [1], owners: ["backend"], files: [], repo: "/repo/api" },
    ]);
  });
});
