// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings } from "@/test/mockBindings";
import { setupUser } from "@/test/ui";
import { workspaceStore } from "../store";
import { mockWorkspace, wp } from "./testSupport";
import { LinkingPolicy } from "./LinkingPolicy";

describe("LinkingPolicy", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("shows the read-only label and sends protected ids", async () => {
    mockWorkspace({ policy: "ReadAndModifyOnRequest", projects: [wp("proj-1"), wp("proj-2", { order: 1 })] });
    const setPolicy = vi.spyOn(workspaceStore, "setPolicy").mockResolvedValue(null);
    render(<LinkingPolicy />);
    expect(screen.getByText("Read only: file tools blocked, shell commands checked")).toBeInTheDocument();
    expect(screen.getByText("Applies to sessions started from now on.")).toBeInTheDocument();
    await setupUser().click(screen.getAllByLabelText("Protected target")[1]);
    expect(setPolicy).toHaveBeenCalledWith("ReadAndModifyOnRequest", ["proj-2"]);
  });

  it("disables the per-project switches under Read only", () => {
    mockWorkspace({ policy: "ReadOnly", projects: [wp("proj-1")] });
    render(<LinkingPolicy />);
    expect(screen.getByLabelText("Protected target")).toBeDisabled();
    expect(screen.getByText("Already read only")).toBeInTheDocument();
  });
});
