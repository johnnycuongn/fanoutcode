import { describe, it, expect, vi, beforeEach } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands, events } from "@/test/mockBindings";
import { makeTask, needsYou, WORKING, IDLE } from "@/test/fixtures";
import {
  tasksStore,
  loadTasks,
  startTasksEvents,
  tasksByState,
  needsYouTasks,
  containerKey,
  projectContainer,
} from "./tasksStore";

const container = projectContainer("proj-a1");

describe("tasksStore", () => {
  beforeEach(() => tasksStore.set({ container: null, byId: {}, order: [], needsYouByContainer: {}, loaded: false }));
  it("loads tasks for a container", async () => {
    commands.tasksList.mockResolvedValue({ status: "ok", data: [makeTask({ id: "task-b2" })] });
    await loadTasks(container);
    expect(commands.tasksList).toHaveBeenCalledWith(container);
    expect(tasksStore.get().order).toEqual(["task-b2"]);
  });
  it("applies task-changed only for its container and inserts unknown tasks", async () => {
    tasksStore.set({ container, loaded: true });
    const stop = startTasksEvents();
    events.taskChanged.emitForTest({ task: makeTask({ id: "task-b2", project_id: "proj-a1", title: "A" }) });
    events.taskChanged.emitForTest({ task: makeTask({ id: "task-ff", project_id: "proj-zz", title: "B" }) });
    expect(Object.keys(tasksStore.get().byId)).toEqual(["task-b2"]);
    stop();
  });
  it("records needs-you counts by container key", () => {
    const stop = startTasksEvents();
    events.needsYou.emitForTest({ container, count: 3 });
    expect(tasksStore.get().needsYouByContainer[containerKey(container)]).toBe(3);
    expect(container).toEqual({ type: "Project", id: "proj-a1" });
    expect(containerKey(container)).toBe("project:proj-a1");
    stop();
  });
  it("groups by state newest first and drops archived", () => {
    const a = makeTask({ id: "task-01", state: needsYou(), updated_at: "2026-10-24T10:00:00Z" });
    const b = makeTask({ id: "task-02", state: needsYou(), updated_at: "2026-10-24T11:00:00Z" });
    const c = makeTask({ id: "task-03", state: WORKING });
    const d = makeTask({ id: "task-04", state: IDLE, archived: true });
    const g = tasksByState([a, b, c, d]);
    expect(g.needsYou.map((t) => t.id)).toEqual(["task-02", "task-01"]);
    expect(g.running.map((t) => t.id)).toEqual(["task-03"]);
    expect(g.idle).toEqual([]);
    expect(needsYouTasks([a, b, c], "proj-a1")[0].id).toBe("task-02");
  });
});
