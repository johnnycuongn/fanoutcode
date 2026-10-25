import { describe, it, expect, vi } from "vitest";
import { projectContainer } from "@/lib/stores/tasksStore";
import { newWorkspaceFromOpenWindows } from "./actions";

describe("newWorkspaceFromOpenWindows", () => {
  it("creates a workspace from distinct open project windows, opens it and closes them", async () => {
    const deps = {
      windowsList: vi.fn().mockResolvedValue([
        { window_id: "w-1", title: "Alpha", container: projectContainer("proj-a1"), project_ids: ["proj-a1"] },
        { window_id: "w-2", title: "Beta", container: projectContainer("proj-b2"), project_ids: ["proj-b2"] },
        { window_id: "w-3", title: "Alpha", container: projectContainer("proj-a1"), project_ids: ["proj-a1"] },
        { window_id: "w-4", title: "Start", container: null, project_ids: [] },
      ]),
      workspacesCreate: vi.fn().mockResolvedValue({
        id: "ws-d4",
        name: "Alpha workspace",
        file_path: "/data/Alpha workspace.fanoutcode-workspace.json",
      }),
      windowsOpenWorkspace: vi.fn().mockResolvedValue(undefined),
      windowsClose: vi.fn().mockResolvedValue(undefined),
    };
    expect(await newWorkspaceFromOpenWindows(deps)).toBe("ws-d4");
    expect(deps.workspacesCreate).toHaveBeenCalledWith("Alpha workspace", ["proj-a1", "proj-b2"]);
    expect(deps.windowsOpenWorkspace).toHaveBeenCalledWith("/data/Alpha workspace.fanoutcode-workspace.json");
    expect(deps.windowsClose.mock.calls.map((c) => c[0])).toEqual(["w-1", "w-2", "w-3"]);
  });
  it("does nothing without project windows", async () => {
    const deps = {
      windowsList: vi.fn().mockResolvedValue([]),
      workspacesCreate: vi.fn(),
      windowsOpenWorkspace: vi.fn(),
      windowsClose: vi.fn(),
    };
    expect(await newWorkspaceFromOpenWindows(deps)).toBeNull();
    expect(deps.workspacesCreate).not.toHaveBeenCalled();
  });
});
