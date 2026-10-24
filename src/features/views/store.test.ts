// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands } from "@/test/mockBindings";
import { makeView, makeTask, needsYou } from "@/test/fixtures";
import { projectContainer } from "@/lib/stores/tasksStore";
import { leaf, leaves, EMPTY_PANE, teamCanvasContent } from "./layout";
import { loadViews } from "./viewsCrud";
import {
  viewsStore,
  splitFocused,
  splitFocusedPane,
  closeFocused,
  setPaneRatio,
  openTaskInView,
  openTeamPane,
  activeView,
} from "./store";
import { tabNeedsYouCount } from "./selectors";
import { shellStore, initWindow } from "@/features/shell";

const container = projectContainer("proj-a1");
const view = makeView({ id: "view-c3", container, layout: leaf("pane-aa", EMPTY_PANE) });

describe("viewsStore", () => {
  beforeEach(async () => {
    vi.useFakeTimers();
    localStorage.clear();
    initWindow({ window_id: "w-1", title: "Alpha", container, project_ids: ["proj-a1"] });
    commands.viewsList.mockResolvedValue({ status: "ok", data: [view] });
    commands.viewsUpdateLayout.mockResolvedValue({ status: "ok", data: null });
    await loadViews(container);
    shellStore.set({ activeTab: "view-c3" });
  });
  it("loads views into the tab order", () => expect(shellStore.get().tabOrder).toEqual(["board", "view-c3"]));
  it("splits the focused pane, focuses the new pane, opens the picker and persists", async () => {
    splitFocused("Horizontal");
    const v = activeView()!;
    expect(v.layout).toMatchObject({ type: "Split", direction: "Horizontal" });
    expect(viewsStore.get().focusedPane["view-c3"]).not.toBe("pane-aa");
    expect(viewsStore.get().pickerFor?.viewId).toBe("view-c3");
    await vi.runAllTimersAsync();
    expect(commands.viewsUpdateLayout).toHaveBeenCalledWith("view-c3", v.layout);
  });
  it("closing the last pane empties it instead of removing it", () => {
    openTaskInView("view-c3", "task-b2");
    closeFocused();
    expect(activeView()!.layout).toEqual(leaf("pane-aa", EMPTY_PANE));
  });
  it("opens one team canvas pane beside a task pane and refocuses it on repeat", () => {
    openTaskInView("view-c3", "task-b2");
    const team = openTeamPane("view-c3", "pane-aa", "task-b2");
    expect(leaves(activeView()!.layout).map((l) => l.content)).toEqual([
      expect.objectContaining({ type: "Task" }),
      teamCanvasContent("task-b2"),
    ]);
    expect(viewsStore.get().focusedPane["view-c3"]).toBe(team);
    expect(viewsStore.get().pickerFor).toBeNull();
    expect(openTeamPane("view-c3", "pane-aa", "task-b2")).toBe(team);
    expect(leaves(activeView()!.layout)).toHaveLength(2);
  });
  it("splitFocusedPane fills the new pane, skips the picker, resolves after persisting and refocuses equal content", async () => {
    openTaskInView("view-c3", "task-b2");
    commands.viewsUpdateLayout.mockClear();
    await splitFocusedPane("Vertical", teamCanvasContent("task-b2"));
    const [first, second] = leaves(activeView()!.layout);
    expect(activeView()!.layout).toMatchObject({ type: "Split", direction: "Vertical" });
    expect(second.content).toEqual(teamCanvasContent("task-b2"));
    expect(viewsStore.get().focusedPane["view-c3"]).toBe(second.pane);
    expect(viewsStore.get().pickerFor).toBeNull();
    expect(commands.viewsUpdateLayout).toHaveBeenCalledWith("view-c3", activeView()!.layout);
    viewsStore.set((s) => ({ ...s, focusedPane: { ...s.focusedPane, "view-c3": first.pane } }));
    await splitFocusedPane("Horizontal", teamCanvasContent("task-b2"));
    expect(leaves(activeView()!.layout)).toHaveLength(2);
    expect(viewsStore.get().focusedPane["view-c3"]).toBe(second.pane);
  });
  it("debounces ratio persistence to one call", async () => {
    splitFocused("Horizontal");
    await vi.runAllTimersAsync();
    commands.viewsUpdateLayout.mockClear();
    setPaneRatio("view-c3", [], 0.3);
    setPaneRatio("view-c3", [], 0.35);
    setPaneRatio("view-c3", [], 0.4);
    await vi.advanceTimersByTimeAsync(299);
    expect(commands.viewsUpdateLayout).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(commands.viewsUpdateLayout).toHaveBeenCalledTimes(1);
  });
  it("restores the active view and focused pane after reload", async () => {
    splitFocused("Horizontal");
    const focused = viewsStore.get().focusedPane["view-c3"];
    await loadViews(container);
    expect(shellStore.get().activeTab).toBe("view-c3");
    expect(viewsStore.get().focusedPane["view-c3"]).toBe(focused);
  });
  it("counts distinct needs-you tasks in a view", () => {
    const t = makeTask({ id: "task-b2", state: needsYou() });
    openTaskInView("view-c3", "task-b2");
    splitFocused("Vertical");
    openTaskInView("view-c3", "task-b2");
    expect(tabNeedsYouCount(activeView()!, { "task-b2": t })).toBe(1);
  });
});
