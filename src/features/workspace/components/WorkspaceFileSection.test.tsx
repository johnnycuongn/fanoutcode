// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
import { mockBindings } from "@/test/mockBindings";
import { mockWorkspace } from "./testSupport";
import { WorkspaceFileSection } from "./WorkspaceFileSection";

describe("WorkspaceFileSection", () => {
  it("shows dirty chip, explicit mode and the reload banner", () => {
    mockWorkspace(
      { dirty: true, file_path: "/w/Shop.fanoutcode-workspace.json" },
      { explicit_save: true, file_missing: false, missing_projects: [] },
      { externalChange: true },
    );
    render(<WorkspaceFileSection />);
    expect(screen.getByText("Unsaved")).toBeInTheDocument();
    expect(screen.getByText(/Saved on ⌘S/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reload" })).toBeInTheDocument();
  });

  it("offers Re-export when the file is missing", () => {
    mockWorkspace({}, { explicit_save: true, file_missing: true, missing_projects: [] });
    render(<WorkspaceFileSection />);
    expect(screen.getByRole("button", { name: "Re-export…" })).toBeInTheDocument();
  });

  it("says the file is written automatically in app data", () => {
    mockWorkspace();
    render(<WorkspaceFileSection />);
    expect(screen.getByText("Written automatically (app data)")).toBeInTheDocument();
  });
});
