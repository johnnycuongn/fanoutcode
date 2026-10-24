// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from "vitest";
import { projectContainer, workspaceContainer } from "@/lib/stores/tasksStore";
import {
  shellStore,
  initWindow,
  setTabOrder,
  cycleTab,
  selectTabIndex,
  toggleSidebar,
  setWindowWidth,
  resolveSidebarOpen,
  toggleProjectFocus,
  boardProject,
} from "./store";

const fresh = () =>
  initWindow({ window_id: "w-1", title: "Alpha", container: projectContainer("proj-a1"), project_ids: ["proj-a1"] });

describe("shellStore", () => {
  beforeEach(() => {
    localStorage.clear();
    fresh();
  });
  it("makes Board tab zero and cycles with wrap", () => {
    setTabOrder(["view-c3", "view-c4"]);
    expect(shellStore.get().tabOrder).toEqual(["board", "view-c3", "view-c4"]);
    cycleTab(-1);
    expect(shellStore.get().activeTab).toBe("view-c4");
    cycleTab(1);
    expect(shellStore.get().activeTab).toBe("board");
  });
  it("selects the Nth tab and ignores out-of-range", () => {
    setTabOrder(["view-c3"]);
    selectTabIndex(2);
    expect(shellStore.get().activeTab).toBe("view-c3");
    selectTabIndex(7);
    expect(shellStore.get().activeTab).toBe("view-c3");
  });
  it("hides the sidebar below 1,100 px only until the user chooses", () => {
    expect(resolveSidebarOpen(null, 1099)).toBe(false);
    expect(resolveSidebarOpen(null, 1100)).toBe(true);
    expect(resolveSidebarOpen(false, 1600)).toBe(false);
    setWindowWidth(900);
    expect(shellStore.get().sidebarOpen).toBe(false);
    toggleSidebar();
    expect(shellStore.get().sidebarOpen).toBe(true);
    setWindowWidth(800);
    expect(shellStore.get().sidebarOpen).toBe(true);
    expect(localStorage.getItem("tc:sidebar:project:proj-a1")).toContain('"open":true');
  });
  it("remembers the section per window", () => {
    shellStore.set({ sidebarSection: "ticket" });
    toggleSidebar();
    fresh();
    expect(shellStore.get().sidebarSection).toBe("ticket");
  });
  it("toggles project focus with ⌘N and clears on repeat", () => {
    initWindow({
      window_id: "w-2",
      title: "WS",
      container: workspaceContainer("ws-d4"),
      project_ids: ["proj-a1", "proj-b2"],
    });
    toggleProjectFocus(2, ["proj-a1", "proj-b2"]);
    expect(shellStore.get().focusedProject).toBe("proj-b2");
    toggleProjectFocus(2, ["proj-a1", "proj-b2"]);
    expect(shellStore.get().focusedProject).toBeNull();
    toggleProjectFocus(5, ["proj-a1", "proj-b2"]);
    expect(shellStore.get().focusedProject).toBeNull();
  });
  it("board shows the focused project only in rail mode", () => {
    shellStore.set({ kind: "workspace", projectLayer: "MultiRoot", focusedProject: "proj-a1" });
    expect(boardProject(shellStore.get(), ["proj-a1"])).toBe("all");
    shellStore.set({ projectLayer: "Rail" });
    expect(boardProject(shellStore.get(), ["proj-a1"])).toBe("proj-a1");
  });
});
