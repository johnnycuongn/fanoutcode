import { describe, it, expect } from "vitest";
import type { Task } from "@/lib/bindings";
import { makeTask, makeView, needsYou, WORKING } from "@/test/fixtures";
import { EMPTY_PANE, leaf, split, taskContent } from "./layout";
import { viewStage, viewStatus } from "./selectors";

const byId = (...tasks: Task[]) => Object.fromEntries(tasks.map((t) => [t.id, t]));
const pair = (a: string, b: string) =>
  makeView({ layout: split("Horizontal", 0.5, leaf("pane-a", taskContent(a)), leaf("pane-b", taskContent(b))) });

describe("viewStatus", () => {
  it("is null when the view shows no task", () => {
    expect(viewStatus(makeView({ layout: leaf("pane-a", EMPTY_PANE) }), {})).toBeNull();
  });
  it("is idle when every task is idle", () => {
    const view = makeView({ layout: leaf("pane-a", taskContent("task-1")) });
    expect(viewStatus(view, byId(makeTask({ id: "task-1" })))).toBe("idle");
  });
  it("is working when a task works and none needs you", () => {
    const tasks = byId(makeTask({ id: "task-1", state: WORKING }), makeTask({ id: "task-2" }));
    expect(viewStatus(pair("task-1", "task-2"), tasks)).toBe("working");
  });
  it("is needs-you when any task needs you, even while another works", () => {
    const tasks = byId(makeTask({ id: "task-1", state: WORKING }), makeTask({ id: "task-2", state: needsYou() }));
    expect(viewStatus(pair("task-1", "task-2"), tasks)).toBe("needs-you");
  });
});

describe("viewStage", () => {
  it("is null when the view shows no task", () => {
    expect(viewStage(makeView({ layout: leaf("pane-a", EMPTY_PANE) }), {})).toBeNull();
  });
  it("is the stage of a single task", () => {
    const view = makeView({ layout: leaf("pane-a", taskContent("task-1")) });
    expect(viewStage(view, byId(makeTask({ id: "task-1", stage: "Build" })))).toBe("Build");
  });
  it("is that stage when several tasks share it", () => {
    const tasks = byId(makeTask({ id: "task-1", stage: "Test" }), makeTask({ id: "task-2", stage: "Test" }));
    expect(viewStage(pair("task-1", "task-2"), tasks)).toBe("Test");
  });
  it("is many when tasks sit in more than one stage", () => {
    const tasks = byId(makeTask({ id: "task-1", stage: "Spec" }), makeTask({ id: "task-2", stage: "Build" }));
    expect(viewStage(pair("task-1", "task-2"), tasks)).toBe("many");
  });
});
