import { describe, it, expect } from "vitest";
import { needsYou, WORKING, IDLE } from "@/test/fixtures";
import { projectContainer, workspaceContainer } from "@/lib/stores/tasksStore";
import { enteredNeedsYou, planNotification, ownsAppWide, badgeTotal } from "./notify";

const ny = needsYou();
describe("notify rules", () => {
  it("fires only on the edge into needs-you", () => {
    expect(enteredNeedsYou(WORKING, ny)).toBe(true);
    expect(enteredNeedsYou(undefined, ny)).toBe(true);
    expect(enteredNeedsYou(ny, needsYou({ type: "PermissionNeeded" }))).toBe(false);
    expect(enteredNeedsYou(ny, IDLE)).toBe(false);
  });
  it("toasts in the focused window when the task is not visible, OS-notifies when nothing is focused", () => {
    expect(
      planNotification({
        entered: true,
        windowFocused: true,
        taskVisibleInActiveView: false,
        anyWindowFocused: true,
        ownsAppWide: true,
      }),
    ).toEqual({ toast: true, os: false });
    expect(
      planNotification({
        entered: true,
        windowFocused: true,
        taskVisibleInActiveView: true,
        anyWindowFocused: true,
        ownsAppWide: true,
      }),
    ).toEqual({ toast: false, os: false });
    expect(
      planNotification({
        entered: true,
        windowFocused: false,
        taskVisibleInActiveView: false,
        anyWindowFocused: false,
        ownsAppWide: true,
      }),
    ).toEqual({ toast: false, os: true });
    expect(
      planNotification({
        entered: true,
        windowFocused: false,
        taskVisibleInActiveView: false,
        anyWindowFocused: false,
        ownsAppWide: false,
      }),
    ).toEqual({ toast: false, os: false });
    expect(
      planNotification({
        entered: false,
        windowFocused: false,
        taskVisibleInActiveView: false,
        anyWindowFocused: false,
        ownsAppWide: true,
      }),
    ).toEqual({ toast: false, os: false });
  });
  it("exactly one window owns app-wide effects", () => {
    expect(ownsAppWide("w-1", ["w-3", "w-1", "w-2"])).toBe(true);
    expect(ownsAppWide("w-2", ["w-3", "w-1", "w-2"])).toBe(false);
  });
  it("totals needs-you over distinct open projects", () => {
    const counts = { "project:proj-a1": 2, "project:proj-b2": 1, "workspace:ws-d4": 3 };
    const windows = [
      { window_id: "w-1", title: "A", container: projectContainer("proj-a1"), project_ids: ["proj-a1"] },
      { window_id: "w-2", title: "WS", container: workspaceContainer("ws-d4"), project_ids: ["proj-a1", "proj-b2"] },
    ];
    expect(badgeTotal(counts, windows)).toBe(3);
  });
});
