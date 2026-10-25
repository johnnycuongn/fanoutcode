import { describe, it, expect } from "vitest";
import { screen } from "@testing-library/react";
import { renderUi } from "@/test/ui";
import { FileCard } from "./FileCard";
import { ROOTS } from "../../fixtures";

const file = {
  path: "/repo/app/src/a.ts",
  repo: "/repo/app",
  added: 3,
  removed: 1,
  owners: ["a1", "a2"],
  sharedAt: 9,
  steps: [1],
  ownerRoles: { a1: "frontend", a2: "backend" },
};
const view = { detail: "Simple" as const, showCost: false, showRepo: true, roots: ROOTS, reduced: true };

describe("FileCard", () => {
  it("shows the short path, net +/-, owner avatars and a warning border when shared", () => {
    const { container } = renderUi(<FileCard data={file} view={view} />);
    expect(screen.getByText("src/a.ts")).toBeTruthy();
    expect(screen.getByText("+3 −1")).toBeTruthy();
    // Avatar initials come from the owner's role (the primitive's two-letter rule).
    expect(screen.getAllByText(/^(FR|BA)$/)).toHaveLength(2);
    expect(screen.getByRole("img", { name: "frontend" })).toBeTruthy();
    const root = container.querySelector('[data-node-kind="file"]')!;
    expect(root.getAttribute("data-shared")).toBe("true");
    expect(root.className).toContain("border-warn");
    expect(screen.getByText("app")).toBeTruthy();
  });
  it("uses the plain border with one owner", () => {
    const { container } = renderUi(<FileCard data={{ ...file, owners: ["a1"], sharedAt: null }} view={view} />);
    expect(container.querySelector('[data-node-kind="file"]')!.className).toContain("border-line");
  });
});
