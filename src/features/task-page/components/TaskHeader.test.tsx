import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import type { CostSummary, Project } from "@/lib/bindings";
import { setupUser } from "@/test/ui";
import { primaryActionFor } from "../primaryAction";
import { makeTask } from "../testing/fixtures";
import type { TaskSettings } from "../types";
import { StageStepper } from "./StageStepper";
import { TaskHeader, type TaskHeaderProps } from "./TaskHeader";

const settings: TaskSettings = {
  mayCommit: false,
  costOverview: false,
  testCommand: null,
  advanceOnGreen: false,
  dockPosition: "Bottom",
  teamViewPlacement: "DockGrows",
};
const props = (over: Partial<TaskHeaderProps> = {}): TaskHeaderProps => ({
  task: makeTask(),
  project: null,
  showProjectBadge: false,
  settings,
  cost: undefined,
  contextPct: undefined,
  primary: primaryActionFor(makeTask(), { mayCommit: false, queued: false }),
  linksChanged: false,
  renaming: false,
  onRenamingChange: vi.fn(),
  onPrimary: vi.fn(),
  onBack: vi.fn(),
  onRename: vi.fn(),
  onAction: vi.fn(),
  onStopAgent: vi.fn(),
  onLinkIssue: vi.fn(),
  ...over,
});

test("the stepper marks earlier stages done and the current one current", () => {
  const { container } = render(<StageStepper stage="Review" />);
  expect([...container.querySelectorAll("[data-state]")].map((e) => e.getAttribute("data-state"))).toEqual([
    "done",
    "done",
    "done",
    "current",
    "upcoming",
    "upcoming",
  ]);
  expect(container.querySelector('[data-state="current"]')?.className).toContain("stage-review");
});
test("Done marks all six chips done", () => {
  const { container } = render(<StageStepper stage="Done" />);
  expect(container.querySelectorAll('[data-state="done"]')).toHaveLength(6);
});
test("the project badge renders only in workspace windows", () => {
  const project = { id: "proj-1", name: "api", color: "teal" } as Project;
  const { rerender } = render(<TaskHeader {...props({ project })} />);
  expect(screen.queryByText("api")).toBeNull();
  rerender(<TaskHeader {...props({ project, showProjectBadge: true })} />);
  expect(screen.getByText("api")).toBeTruthy();
});
test("cost and the context meter render only with the overview on", () => {
  const cost = { cost_usd: 1, input_tokens: 1000, output_tokens: 0, estimated: false } as CostSummary;
  const { rerender } = render(<TaskHeader {...props({ cost, contextPct: 40 })} />);
  expect(screen.queryByText("$1.00 · 1.0k tokens")).toBeNull();
  rerender(<TaskHeader {...props({ cost, contextPct: 40, settings: { ...settings, costOverview: true } })} />);
  expect(screen.getByText("$1.00 · 1.0k tokens")).toBeTruthy();
  expect(screen.getByRole("meter", { name: "Context window" }).getAttribute("aria-valuenow")).toBe("40");
});
test("clicking the title opens inline rename; Enter commits, Escape cancels", () => {
  const onRename = vi.fn();
  const onRenamingChange = vi.fn();
  const { rerender } = render(<TaskHeader {...props({ onRename, onRenamingChange })} />);
  fireEvent.click(screen.getByRole("button", { name: /rename/i }));
  expect(onRenamingChange).toHaveBeenCalledWith(true);
  rerender(<TaskHeader {...props({ onRename, onRenamingChange, renaming: true })} />);
  const input = screen.getByRole("textbox", { name: "Task title" });
  fireEvent.change(input, { target: { value: "Better" } });
  fireEvent.keyDown(input, { key: "Enter" });
  expect(onRename).toHaveBeenCalledWith("Better");
  fireEvent.keyDown(input, { key: "Escape" });
  expect(onRenamingChange).toHaveBeenLastCalledWith(false);
});
test("the primary button shows the label, the queued hint and Start fresh session", async () => {
  const primary = primaryActionFor(makeTask({ state: { type: "Working" } }), { mayCommit: false, queued: true });
  const onAction = vi.fn();
  render(<TaskHeader {...props({ primary, onAction })} />);
  expect(screen.getByRole("button", { name: "Start build" })).toBeTruthy();
  expect(screen.getByText("Will send when idle")).toBeTruthy();
  // Radix opens its menu on pointerdown or a key, not a synthetic click
  screen.getByRole("button", { name: "More actions" }).focus();
  const user = setupUser();
  await user.keyboard("{Enter}");
  await user.click(await screen.findByRole("menuitem", { name: "Start fresh session" }));
  expect(onAction).toHaveBeenCalledWith({ type: "StartFresh" });
});
test("the state chip shows handbackLabel's text and is absent when no hand-back is pending", () => {
  const planDone = makeTask({ state: { type: "NeedsYou", reason: { type: "StageDone" } } });
  const { rerender } = render(<TaskHeader {...props({ task: planDone })} />);
  expect(screen.getByTestId("handback-chip").textContent).toBe("Plan written, go?"); // A2 canonical text
  const replied = makeTask({ state: { type: "NeedsYou", reason: { type: "Replied", first_line: "Which API?" } } });
  rerender(<TaskHeader {...props({ task: replied })} />);
  expect(screen.getByText("Which API?")).toBeTruthy();
  rerender(<TaskHeader {...props()} />);
  expect(screen.queryByTestId("handback-chip")).toBeNull();
});
test("the links-changed hint appears when asked", () => {
  render(<TaskHeader {...props({ linksChanged: true })} />);
  expect(screen.getByText("Links changed, start a fresh session to apply")).toBeTruthy();
});
