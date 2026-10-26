import { act, render, screen, waitFor } from "@testing-library/react";
import { useEffect, type ReactElement } from "react";
import { expect, test, vi } from "vitest";
import type { Stage, StageAction } from "@/lib/bindings";
import { useForwardGuard, type GuardTests } from "./useForwardGuard";

const h = vi.hoisted(() => ({
  pending: 0,
  review: null as string | null,
  detected: null as { command: string; source: string } | null,
  settingsSet: vi.fn(async () => ({ status: "ok", data: null })),
}));
vi.mock("@/lib/bindings", () => ({
  commands: {
    gitUncommitted: vi.fn(async () => ({ status: "ok", data: h.pending })),
    memoryRead: vi.fn(async () => ({ status: "ok", data: h.review })),
    projectsDetectTest: vi.fn(async () => ({ status: "ok", data: h.detected })),
    settingsSet: h.settingsSet,
  },
  events: {
    gitChanges: { listen: vi.fn(async () => () => {}) },
    taskChanged: { listen: vi.fn(async () => () => {}) },
  },
}));

const advance: StageAction = { type: "Advance" };
const approve: StageAction = { type: "ApproveChanges" };
type Guard = ReturnType<typeof useForwardGuard>;
let g: Guard | null = null;
const keep = (x: Guard) => {
  g = x;
};

function Host({
  stage,
  onGuard,
  tests = { command: "npm test", enabled: true },
}: {
  stage: Stage;
  onGuard(g: Guard): void;
  tests?: GuardTests;
}): ReactElement {
  const guard = useForwardGuard({ id: "task-1", stage, project_id: "proj-1" }, tests);
  useEffect(() => onGuard(guard));
  return <>{guard.dialog}</>;
}

/** Starts the guard and returns its pending answer boxed: an async function would adopt (and wait for) the promise. */
async function ask(action: StageAction): Promise<{ answer: Promise<boolean> }> {
  let answer: Promise<boolean> = Promise.resolve(false);
  await act(async () => {
    answer = g!.guard(action);
  });
  return { answer };
}

test("Mark done with nothing uncommitted goes ahead without asking", async () => {
  h.pending = 0;
  render(<Host stage="Commit" onGuard={keep} />);
  await expect(g!.guard(advance)).resolves.toBe(true);
  expect(screen.queryByRole("dialog")).toBeNull();
});

test("Mark done with uncommitted files asks, and Cancel keeps the task in Commit", async () => {
  h.pending = 2;
  render(<Host stage="Commit" onGuard={keep} />);
  const { answer } = await ask(advance);
  expect(await screen.findByText("Mark done with uncommitted changes?")).toBeTruthy();
  expect(screen.getByText(/2 files from this task are not committed yet/)).toBeTruthy();
  act(() => screen.getByRole("button", { name: "Cancel" }).click());
  await expect(answer).resolves.toBe(false);
});

test("Mark done anyway goes ahead, and other actions are never asked about", async () => {
  h.pending = 1;
  render(<Host stage="Commit" onGuard={keep} />);
  const { answer } = await ask(advance);
  act(() => screen.getByRole("button", { name: "Mark done anyway" }).click());
  await expect(answer).resolves.toBe(true);
  await expect(g!.guard({ type: "Commit" } as StageAction)).resolves.toBe(true);
});

test("a blocker in review.md shows on the button and Approve changes asks first", async () => {
  h.review = [
    "## best-practice",
    "- app/src/streak.js:8 · blocker · Steps back a day in local time.",
    "- app/src/cli.js:6 · minor · Records the UTC date.",
  ].join("\n");
  render(<Host stage="Review" onGuard={keep} />);
  await waitFor(() => expect(g!.blockers).toBe(1));
  const { answer } = await ask(approve);
  expect(await screen.findByText("Approve with 1 blocker in review?")).toBeTruthy();
  expect(screen.getByText(/starting with: Steps back a day in local time/)).toBeTruthy();
  act(() => screen.getByRole("button", { name: "Approve anyway" }).click());
  await expect(answer).resolves.toBe(true);
});

test("a review with no blockers approves without asking", async () => {
  h.review = "- app/src/cli.js:6 · minor · Records the UTC date.";
  render(<Host stage="Review" onGuard={keep} />);
  await expect(g!.guard(approve)).resolves.toBe(true);
  expect(g!.blockers).toBe(0);
});

test("with no test command, Approve changes offers the detected one and Use saves it", async () => {
  h.review = null;
  h.detected = { command: "npm test", source: "package.json" };
  h.settingsSet.mockClear();
  render(<Host stage="Review" onGuard={keep} tests={{ command: null, enabled: true }} />);
  const { answer } = await ask(approve);
  expect(await screen.findByText("Run the tests with npm test?")).toBeTruthy();
  expect(screen.getByText(/runs this repository's own script/)).toBeTruthy();
  act(() => screen.getByRole("button", { name: "Use npm test" }).click());
  await expect(answer).resolves.toBe(true);
  expect(h.settingsSet).toHaveBeenCalledWith(
    { workspace: null, project: "proj-1", task: null },
    "test_command",
    "npm test",
  );
});

test("Skip tests approves without saving, and a saved command or tests off never asks", async () => {
  h.review = null;
  h.detected = { command: "npm test", source: "package.json" };
  h.settingsSet.mockClear();
  const { unmount } = render(<Host stage="Review" onGuard={keep} tests={{ command: null, enabled: true }} />);
  const { answer } = await ask(approve);
  act(() => screen.getByRole("button", { name: "Skip tests" }).click());
  await expect(answer).resolves.toBe(true);
  expect(h.settingsSet).not.toHaveBeenCalled();
  unmount();
  render(<Host stage="Review" onGuard={keep} tests={{ command: null, enabled: false }} />);
  await expect(g!.guard(approve)).resolves.toBe(true);
  expect(screen.queryByRole("dialog")).toBeNull();
});
