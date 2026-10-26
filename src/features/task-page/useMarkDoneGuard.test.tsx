import { act, render, screen } from "@testing-library/react";
import { useEffect, type ReactElement } from "react";
import { expect, test, vi } from "vitest";
import type { Stage, StageAction } from "@/lib/bindings";
import { useMarkDoneGuard } from "./useMarkDoneGuard";

const h = vi.hoisted(() => ({ pending: 0 }));
vi.mock("@/lib/bindings", () => ({
  commands: { gitUncommitted: vi.fn(async () => ({ status: "ok", data: h.pending })) },
  events: { gitChanges: { listen: vi.fn(async () => () => {}) } },
}));

const advance: StageAction = { type: "Advance" };
type Guard = (a: StageAction) => Promise<boolean>;
let guard: Guard = async () => false;
const keep = (g: Guard) => {
  guard = g;
};

function Host({ stage, onGuard }: { stage: Stage; onGuard(g: Guard): void }): ReactElement {
  const g = useMarkDoneGuard({ id: "task-1", stage });
  useEffect(() => onGuard(g.guard), [g.guard, onGuard]);
  return <>{g.dialog}</>;
}

test("Mark done with nothing uncommitted goes ahead without asking", async () => {
  h.pending = 0;
  render(<Host stage="Commit" onGuard={keep} />);
  await expect(guard(advance)).resolves.toBe(true);
  expect(screen.queryByRole("dialog")).toBeNull();
});

test("Mark done with uncommitted files asks, and Cancel keeps the task in Commit", async () => {
  h.pending = 2;
  render(<Host stage="Commit" onGuard={keep} />);
  let answer: Promise<boolean> = Promise.resolve(true);
  await act(async () => {
    answer = guard(advance);
  });
  expect(await screen.findByText("Mark done with uncommitted changes?")).toBeTruthy();
  expect(screen.getByText(/2 files from this task are not committed yet/)).toBeTruthy();
  act(() => screen.getByRole("button", { name: "Cancel" }).click());
  await expect(answer).resolves.toBe(false);
});

test("Mark done anyway goes ahead, and other stages and actions are never asked about", async () => {
  h.pending = 1;
  render(<Host stage="Commit" onGuard={keep} />);
  let answer: Promise<boolean> = Promise.resolve(false);
  await act(async () => {
    answer = guard(advance);
  });
  act(() => screen.getByRole("button", { name: "Mark done anyway" }).click());
  await expect(answer).resolves.toBe(true);
  await expect(guard({ type: "Commit" } as StageAction)).resolves.toBe(true);
});
