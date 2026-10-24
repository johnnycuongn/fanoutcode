import { describe, expect, it, vi } from "vitest";
import { SWATCHES } from "@/lib/theme";
import { WORKTREE_LOCKED_HINT, cardMenuItems, type MenuCtx } from "./menu";
import { WORKING, makeTask, projectContainer, workspaceContainer } from "./testing/fixtures";

const ctx: MenuCtx = {
  windows: [
    { window_id: "w-1", title: "Alpha", container: projectContainer("p-1"), project_ids: ["p-1"] },
    { window_id: "w-2", title: "Workspace", container: workspaceContainer("ws"), project_ids: ["p-1", "p-2"] },
    { window_id: "w-3", title: "Other", container: projectContainer("p-9"), project_ids: ["p-9"] },
  ],
  currentWindowId: "w-1",
  onOpen: vi.fn(),
  confirm: vi.fn(async () => true),
  openJira: vi.fn(),
};
const ids = (t = makeTask()) => cardMenuItems(t, ctx).map((i) => i.id);
const item = (t: ReturnType<typeof makeTask>, id: string) => cardMenuItems(t, ctx).find((i) => i.id === id)!;

describe("cardMenuItems", () => {
  it("lists every item in spec order with its shortcut", () => {
    expect(ids()).toEqual([
      "open",
      "open_new_view",
      "open_window",
      "rename",
      "priority",
      "color",
      "flag",
      "hide",
      "archive",
      "link_jira",
      "worktree",
      "stop_agent",
    ]);
    expect(item(makeTask(), "rename").shortcut).toBe("⌘E");
    expect(item(makeTask(), "archive").shortcut).toBe("⌘⇧A");
    expect(item(makeTask(), "open_new_view").shortcut).toBe("⌘ click");
  });
  it("offers only other windows that contain the task home project under Open in window", () => {
    expect(item(makeTask({ project_id: "p-1" }), "open_window").children!.map((c) => c.label)).toEqual(["Workspace"]);
  });
  it("Open in window is disabled with a hint when no other window shows the project", () => {
    expect(item(makeTask({ project_id: "p-5" }), "open_window")).toMatchObject({
      disabled: true,
      hint: "No other window shows this project",
      children: [],
    });
  });
  it("worktree is enabled before the session starts and disabled with a hint after", () => {
    expect(item(makeTask({ primary_session: null }), "worktree")).toMatchObject({ disabled: false, checked: false });
    expect(item(makeTask({ primary_session: "s-1", worktree: "On" }), "worktree")).toMatchObject({
      disabled: true,
      checked: true,
      hint: WORKTREE_LOCKED_HINT,
    });
  });
  it("archive becomes Restore for an archived task", () => {
    expect(item(makeTask({ archived: true }), "archive").label).toBe("Restore");
  });
  it("stop agent is disabled when the task has no primary session", () => {
    expect(item(makeTask({ primary_session: null }), "stop_agent").disabled).toBe(true);
    expect(item(makeTask({ primary_session: "s-1", state: WORKING }), "stop_agent")).toMatchObject({
      disabled: false,
      danger: true,
    });
  });
  it("priority and colour submenus mark the current value checked", () => {
    const pri = item(makeTask({ priority: "High" }), "priority").children!;
    expect(pri.map((c) => c.label)).toEqual(["None", "Low", "Medium", "High"]);
    expect(pri.find((c) => c.label === "High")!.checked).toBe(true);
    const col = item(makeTask({ color: SWATCHES[0] }), "color").children!;
    expect(col[0]!.label).toBe("None");
    expect(col.find((c) => c.id === `color:${SWATCHES[0]}`)!.checked).toBe(true);
    expect(col).toHaveLength(SWATCHES.length + 1);
  });
  it("flag and hide labels flip with the task", () => {
    expect(item(makeTask({ flagged: true, hidden: true }), "flag").label).toBe("Unflag");
    expect(item(makeTask({ flagged: true, hidden: true }), "hide").label).toBe("Unhide");
  });
  it("link Jira reads Change when the task is already linked", () => {
    expect(item(makeTask({ jira_key: null }), "link_jira").label).toBe("Link Jira issue…");
    expect(item(makeTask({ jira_key: "TC-1" }), "link_jira").label).toBe("Change Jira issue…");
  });
});
