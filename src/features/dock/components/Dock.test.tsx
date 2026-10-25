import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
vi.mock("@/lib/bindings", async () => (await import("@/test/mockBindings")).mockBindings());
import { commands } from "@/test/mockBindings";
import type { OverlapRow } from "@/lib/bindings";
import { overlapStore } from "@/features/overlap";
import { makeSettings, makeTask } from "@/test/taskPageFixtures";
import { createDockStore } from "../store";
import { mem } from "../testing/fakes";
import { Dock } from "./Dock";

test("renders only the visible tabs for the stage and the team slot on Team", () => {
  const d = createDockStore(mem());
  d.selectTab("Build", "team");
  render(
    <Dock
      task={makeTask({ stage: "Build" })}
      settings={makeSettings()}
      open
      teamSlot={<div data-testid="team" />}
      store={d}
    />,
  );
  expect(screen.getAllByRole("tab").map((t) => t.textContent)).toEqual(["Memory", "Team", "Changes", "Ticket"]);
  expect(screen.getByTestId("team")).toBeTruthy();
});
test("renders nothing when closed and marks the right position", () => {
  const d = createDockStore(mem());
  const { container, rerender } = render(
    <Dock task={makeTask()} settings={makeSettings()} open={false} teamSlot={null} store={d} />,
  );
  expect(container.firstChild).toBeNull();
  rerender(
    <Dock task={makeTask()} settings={makeSettings({ dockPosition: "Right" })} open teamSlot={null} store={d} />,
  );
  expect(container.querySelector('[data-position="right"]')).toBeTruthy();
});

function overlapRow(over: Partial<OverlapRow> = {}): OverlapRow {
  return {
    task_id: "task-1",
    other_task: "task-2",
    repo_root: "/r",
    path: "src/a.ts",
    relation: "Claimed",
    pending: null,
    held: false,
    noted_out: false,
    noted_in: false,
    decided_at: null,
    updated_at: "2026-10-25T08:00:00Z",
    ...over,
  };
}
const setCounts = (count: number, pending: number) =>
  act(() => overlapStore.set((s) => ({ ...s, counts: { ...s.counts, "task-1": { count, pending } } })));

beforeEach(() => {
  overlapStore.reset();
  commands.overlapList.mockReset();
  commands.gitDiff.mockResolvedValue({
    status: "ok",
    data: { repo_root: "/r", path: "src/a.ts", status: "Modified", before: "", after: "x\n", binary: false },
  });
});

test("shows Overlap with the count and amber while pending", async () => {
  commands.overlapList.mockResolvedValue({ status: "ok", data: [overlapRow(), overlapRow({ path: "src/b.ts" })] });
  const d = createDockStore(mem());
  render(<Dock task={makeTask({ stage: "Plan" })} settings={makeSettings()} open teamSlot={null} store={d} />);
  const tab = await screen.findByRole("tab", { name: /Overlap/ });
  expect(commands.overlapList).toHaveBeenCalledWith("task-1");
  expect(within(tab).getByText("2").className).toContain("text-mute");
  setCounts(2, 1);
  expect(within(screen.getByRole("tab", { name: /Overlap/ })).getByText("2").className).toContain("text-warn");
  setCounts(0, 0);
  expect(screen.queryByRole("tab", { name: /Overlap/ })).toBeNull();
});

test("selects Overlap once when a decision becomes pending", async () => {
  commands.overlapList.mockResolvedValue({ status: "ok", data: [overlapRow()] });
  const d = createDockStore(mem());
  d.selectTab("Build", "team");
  render(<Dock task={makeTask({ stage: "Build" })} settings={makeSettings()} open teamSlot={null} store={d} />);
  await screen.findByRole("tab", { name: /Overlap/ });
  expect(screen.getByRole("tab", { name: "Team" })).toHaveAttribute("data-state", "active");

  setCounts(1, 1);
  expect(screen.getByRole("tab", { name: /Overlap/ })).toHaveAttribute("data-state", "active");
  expect(await screen.findByRole("heading", { name: /is also changing src\/a\.ts/ })).toBeTruthy();
  expect(d.store.get().tabByStage.Build).toBe("team");

  fireEvent.mouseDown(screen.getByRole("tab", { name: "Team" }));
  expect(screen.getByRole("tab", { name: "Team" })).toHaveAttribute("data-state", "active");
  setCounts(2, 2);
  expect(screen.getByRole("tab", { name: "Team" })).toHaveAttribute("data-state", "active");

  setCounts(1, 0);
  setCounts(1, 1);
  expect(screen.getByRole("tab", { name: /Overlap/ })).toHaveAttribute("data-state", "active");
});

test("a merge that needs you shows Overlap even with no overlap rows", async () => {
  commands.overlapList.mockResolvedValue({ status: "ok", data: [] });
  const d = createDockStore(mem());
  const merging = makeTask({
    stage: "Done",
    state: { type: "NeedsYou", reason: { type: "MergeNeedsYou", repo: "/r", paths: ["src/a.ts"] } },
  });
  render(<Dock task={merging} settings={makeSettings()} open teamSlot={null} store={d} />);
  expect(await screen.findByRole("tab", { name: /Overlap/ })).toBeTruthy();
});
