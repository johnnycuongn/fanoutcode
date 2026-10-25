// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands } from "@/test/mockBindings";
import type { WorkspaceImportPreview } from "@/lib/bindings";
import { renderUi, setupUser } from "@/test/ui";
import { importStore, openWorkspaceFile } from "../importStore";
import { WorkspaceImportDialog } from "./WorkspaceImportDialog";

const ok = <T,>(data: T) => ({ status: "ok" as const, data });
function preview(over: Partial<WorkspaceImportPreview> = {}): WorkspaceImportPreview {
  return {
    name: "Shared",
    roots: [{ path: "/w/api", exists: true, is_dir: true, new_to_app: true, protected: false, warning: null }],
    policy: "ReadAndModifyOnRequest",
    agent_names: ["docs-writer"],
    fragment_stages: ["plan"],
    settings_keys: [],
    docs: [
      {
        id: "doc-1",
        kind: "File",
        target: "/Users/j/.ssh/id_rsa",
        label: "id_rsa",
        page_id: null,
        host: null,
        needs_approval: true,
        outside_projects: true,
      },
    ],
    problems: [],
    blocked: false,
    needs_confirmation: true,
    token: "tok-1",
    ...over,
  };
}

describe("workspace import", () => {
  beforeEach(() => {
    importStore.reset();
    vi.clearAllMocks();
  });

  it("opens a trusted file directly", async () => {
    commands.workspacesPreviewOpen.mockResolvedValue(ok(preview({ needs_confirmation: false })));
    commands.windowsOpenWorkspace.mockResolvedValue(ok({}));
    await openWorkspaceFile("/w/Shop.fanoutcode-workspace.json");
    expect(commands.workspacesConfirmOpen).not.toHaveBeenCalled();
    expect(commands.windowsOpenWorkspace).toHaveBeenCalledWith("/w/Shop.fanoutcode-workspace.json");
  });

  it("shows roots, policy in plain words, additions and per-doc approval, then opens", async () => {
    commands.workspacesPreviewOpen.mockResolvedValue(ok(preview()));
    commands.workspacesConfirmOpen.mockResolvedValue(ok({}));
    commands.windowsOpenWorkspace.mockResolvedValue(ok({}));
    await openWorkspaceFile("/d/Shared.fanoutcode-workspace.json");
    renderUi(<WorkspaceImportDialog />);
    expect(screen.getByText("/w/api")).toBeInTheDocument();
    expect(screen.getByText("Agents can read and edit these folders")).toBeInTheDocument();
    expect(screen.getByText(/custom agents: docs-writer/i)).toBeInTheDocument();
    expect(screen.getByText(/Outside the workspace's projects/)).toBeInTheDocument();
    const user = setupUser();
    await user.click(screen.getByLabelText("Allow reading /Users/j/.ssh/id_rsa"));
    await user.click(screen.getByRole("button", { name: "Open" }));
    expect(commands.workspacesConfirmOpen).toHaveBeenCalledWith("tok-1", ["doc-1"]);
    expect(commands.windowsOpenWorkspace).toHaveBeenCalledWith("/d/Shared.fanoutcode-workspace.json");
  });

  it("cannot open a blocked file", async () => {
    commands.workspacesPreviewOpen.mockResolvedValue(ok(preview({ blocked: true, problems: ["/ is the whole disk"] })));
    await openWorkspaceFile("/d/Bad.fanoutcode-workspace.json");
    render(<WorkspaceImportDialog />);
    expect(screen.getByText("/ is the whole disk")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open" })).toBeDisabled();
  });
});
