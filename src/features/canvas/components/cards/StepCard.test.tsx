import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { StepCard } from "./StepCard";
import { ROOTS } from "../../fixtures";
import type { StepNodeData, ViewOptions } from "../../types";

const step: StepNodeData = {
  number: 2,
  title: "Toolbar",
  status: "active",
  working: true,
  owners: ["ui-polish"],
  after: [],
  repo: "/repo/app",
  files: [
    { path: "/repo/app/src/toolbar.tsx", added: 4, removed: 1, shared: false },
    { path: "/repo/app/src/a.ts", added: 1, removed: 1, shared: true },
  ],
  toolLine: "Edit /repo/app/src/toolbar.tsx",
  progress: 0.5,
  costUsd: 0.21,
  stackIndex: null,
};
const view = (over: Partial<ViewOptions> = {}): ViewOptions => ({
  detail: "Simple",
  showCost: true,
  showRepo: false,
  roots: ROOTS,
  reduced: true,
  ...over,
});

describe("StepCard", () => {
  it("simple shows number, title, owner, status and file count without file rows", () => {
    render(<StepCard data={step} view={view()} />);
    expect(screen.getByText("2.")).toBeTruthy();
    expect(screen.getByText("Toolbar")).toBeTruthy();
    expect(screen.getByText("ui-polish")).toBeTruthy();
    expect(screen.getByText("2 files")).toBeTruthy();
    expect(screen.queryByText(/toolbar\.tsx/)).toBeNull();
  });
  it("medium lists files with +/- and flags shared ones", () => {
    render(<StepCard data={step} view={view({ detail: "Medium" })} />);
    expect(screen.getByText(/toolbar\.tsx/).textContent).toContain("+4 −1");
    expect(screen.getByText(/a\.ts/).className).toContain("text-warn");
    expect(screen.queryByRole("progressbar")).toBeNull();
  });
  it("detailed adds the tool line, a progress bar and cost", () => {
    render(<StepCard data={step} view={view({ detail: "Detailed" })} />);
    expect(screen.getByText("Edit /repo/app/src/toolbar.tsx")).toBeTruthy();
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("50");
    expect(screen.getByText("$0.21")).toBeTruthy();
  });
  it("hides cost when the overview is off", () => {
    render(<StepCard data={step} view={view({ detail: "Detailed", showCost: false })} />);
    expect(screen.queryByText("$0.21")).toBeNull();
  });
  it("collapses a done step to its title line at any detail", () => {
    render(<StepCard data={{ ...step, status: "done" }} view={view({ detail: "Detailed" })} />);
    expect(screen.getByText("Toolbar")).toBeTruthy();
    expect(screen.queryByText("2 files")).toBeNull();
    expect(screen.queryByRole("progressbar")).toBeNull();
  });
  it("shows the repo badge only in workspace tasks", () => {
    const { rerender } = render(<StepCard data={step} view={view({ showRepo: true })} />);
    expect(screen.getByText("app")).toBeTruthy();
    rerender(<StepCard data={step} view={view({ showRepo: false })} />);
    expect(screen.queryByText("app")).toBeNull();
  });
  it("labels the top of the done stack with its count", () => {
    render(<StepCard data={{ ...step, status: "done", stackIndex: 0, stackCount: 2 }} view={view()} />);
    expect(screen.getByText("2 done")).toBeTruthy();
  });
});
