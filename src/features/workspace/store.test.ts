import { describe, it, expect, vi, beforeEach } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands, events } from "@/test/mockBindings";
import type { Workspace } from "@/lib/bindings";
import { workspaceStore } from "./store";

const ok = <T>(data: T) => ({ status: "ok" as const, data });
function ws(id: string, over: Partial<Workspace> = {}): Workspace {
  return {
    id,
    name: "Shop",
    file_path: "/app/ws/Shop.fanoutcode-workspace.json",
    dirty: false,
    projects: [],
    policy: "ReadAndModifyOnRequest",
    ...over,
  };
}
const status = { file_missing: false, missing_projects: [], explicit_save: false };
async function loaded(id: string, over: Partial<Workspace> = {}) {
  commands.workspacesList.mockResolvedValue(ok([ws(id, over)]));
  commands.workspacesStatus.mockResolvedValue(ok(status));
  commands.workspacesFragments.mockResolvedValue(ok({}));
  commands.workspacesAgents.mockResolvedValue(ok({}));
  await workspaceStore.load(id);
}
const emit = (name: string, payload: unknown) => events[name].emitForTest(payload);

describe("workspaceStore", () => {
  beforeEach(() => {
    workspaceStore.reset();
    vi.clearAllMocks();
  });

  it("load fetches workspace, status, fragments and agents", async () => {
    commands.workspacesList.mockResolvedValue(ok([ws("ws-1")]));
    commands.workspacesStatus.mockResolvedValue(ok(status));
    commands.workspacesFragments.mockResolvedValue(ok({ plan: "x" }));
    commands.workspacesAgents.mockResolvedValue(ok({}));
    await workspaceStore.load("ws-1");
    expect(workspaceStore.get().workspace?.id).toBe("ws-1");
    expect(workspaceStore.get().fragments.plan).toBe("x");
    expect(workspaceStore.isExplicitSave()).toBe(false);
  });

  it("workspace-changed replaces only the matching workspace", async () => {
    await loaded("ws-1");
    emit("workspaceChanged", { workspace: ws("ws-2", { dirty: true }) });
    expect(workspaceStore.isDirty()).toBe(false);
    emit("workspaceChanged", { workspace: ws("ws-1", { dirty: true }) });
    expect(workspaceStore.isDirty()).toBe(true);
  });

  it("save clears dirty from the returned workspace", async () => {
    await loaded("ws-1", { dirty: true });
    commands.workspacesSave.mockResolvedValue(ok(ws("ws-1", { dirty: false })));
    await workspaceStore.save();
    expect(commands.workspacesSave).toHaveBeenCalledWith("ws-1");
    expect(workspaceStore.isDirty()).toBe(false);
  });

  it("file-changed event sets externalChange and reload clears it", async () => {
    await loaded("ws-1");
    emit("workspaceFileChanged", { workspace_id: "ws-1" });
    expect(workspaceStore.get().externalChange).toBe(true);
    commands.workspacesReload.mockResolvedValue(ok(ws("ws-1")));
    await workspaceStore.reload(true);
    expect(commands.workspacesReload).toHaveBeenCalledWith("ws-1", true);
    expect(workspaceStore.get().externalChange).toBe(false);
  });

  it("setPolicy sends policy and protected ids", async () => {
    await loaded("ws-1");
    commands.workspacesSetPolicy.mockResolvedValue(ok(ws("ws-1")));
    await workspaceStore.setPolicy("ReadOnly", ["proj-2"]);
    expect(commands.workspacesSetPolicy).toHaveBeenCalledWith("ws-1", "ReadOnly", ["proj-2"]);
  });

  it("an error is kept for the screen and the state is unchanged", async () => {
    await loaded("ws-1");
    commands.workspacesSave.mockResolvedValue({ status: "error", error: "Disk full" });
    await workspaceStore.save();
    expect(workspaceStore.get().error).toBe("Disk full");
    expect(workspaceStore.get().busy).toBe(false);
  });
});
