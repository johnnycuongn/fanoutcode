import { act, render, screen, waitFor } from "@testing-library/react";
import { createRef, forwardRef, useImperativeHandle } from "react";
import { afterEach, beforeEach, expect, test, vi, type Mock } from "vitest";
import type { Container, Task } from "@/lib/bindings";

const h = vi.hoisted(() => {
  const resolved = {
    may_commit: false,
    cost_overview: false,
    test_command: null,
    advance_on_green: false,
    dock_position: "Bottom",
    team_view_placement: "DockGrows",
    livespec_level: "Simple",
  };
  return { focusSpy: vi.fn(), resolved, fakeCommands: {} as Record<string, Mock> };
});
vi.mock("@/features/views", () => ({
  TerminalPane: forwardRef((_p, ref) => {
    useImperativeHandle(ref, () => ({ focus: h.focusSpy }));
    return <div data-testid="terminal" />;
  }),
}));
vi.mock("@/lib/bindings", async () => {
  const { fakeEvents, makeTask } = await import("@/test/taskPageFixtures");
  Object.assign(h.fakeCommands, {
    tasksGet: vi.fn(async () => makeTask()),
    settingsGet: vi.fn(async () => h.resolved),
    memoryRead: vi.fn(async () => ""),
    memoryWrite: vi.fn(async () => null),
    memoryMissing: vi.fn(async () => []),
    gitChanges: vi.fn(async () => []),
    tasksAction: vi.fn(async () => {}),
    sessionsOpen: vi.fn(async () => "sess-1"),
    costSummary: vi.fn(async () => ({})),
    livespecAnalyze: vi.fn(async () => null),
    livespecCancel: vi.fn(async () => null),
  });
  return { commands: h.fakeCommands, events: fakeEvents };
});
vi.mock("../taskSource", async () => ({ tasksSource: (await import("../testing/fixtures")).makeTaskSource() })); // stands in for Plan 06's tasksStore

import { dockStore } from "@/features/dock";
import { specStore } from "@/features/spec-editor";
import { taskPageStore } from "../store";
import { tasksSource } from "../taskSource";
import { makeTask } from "../testing/fixtures";
import { TaskPage, type TaskPageHandle } from "./TaskPage";

const { focusSpy, fakeCommands, resolved } = h;
beforeEach(() => vi.clearAllMocks());
afterEach(() => {
  dockStore.store.reset();
  specStore.store.reset();
  taskPageStore.store.reset();
});
const home = { type: "Project", id: "proj-1" } as Container;
const open = async (task: Task) => {
  tasksSource.put(task);
  const ref = createRef<TaskPageHandle>();
  render(<TaskPage ref={ref} taskId="task-1" paneId="pane-1" container={home} projects={[]} onBack={() => {}} />);
  await screen.findByText(task.title);
  return ref;
};

test("Spec renders the editor with no dock open", async () => {
  await open(makeTask({ stage: "Spec", primary_session: null }));
  expect(await screen.findByRole("radio", { name: "Simple" })).toBeTruthy();
  expect(screen.queryByRole("tab", { name: "Memory" })).toBeNull();
});
test("Plan renders the terminal and the dock on Memory", async () => {
  await open(makeTask({ stage: "Plan" }));
  expect(screen.getByTestId("terminal")).toBeTruthy();
  expect(screen.getByRole("tab", { name: "Memory", selected: true })).toBeTruthy();
  await waitFor(() => expect(fakeCommands.sessionsOpen).toHaveBeenCalledWith("task-1")); // Plan 04: the page opens the session
});
test("the shared task moving into Build switches the dock to Team and grows it", async () => {
  await open(makeTask({ stage: "Plan" }));
  act(() => tasksSource.put(makeTask({ stage: "Build", state: { type: "Working" } })));
  expect(screen.getByRole("tab", { name: "Team", selected: true })).toBeTruthy();
  expect(dockStore.isGrown("Build")).toBe(true);
});
test("in Spec the primary action flushes the spec before sending Approve", async () => {
  const ref = await open(makeTask({ stage: "Spec", primary_session: null }));
  await screen.findByRole("radio", { name: "Simple", checked: true }); // spec.md is loaded
  act(() => specStore.setText("task-1", "A spec long enough to matter."));
  await act(() => ref.current!.primaryAction());
  expect(fakeCommands.memoryWrite).toHaveBeenCalledWith("task-1", "spec.md", "A spec long enough to matter.");
  expect(fakeCommands.tasksAction).toHaveBeenCalledWith("task-1", { type: "Approve" });
});
test("the handle runs the primary action and focuses the terminal in the Replied state", async () => {
  const ref = await open(
    makeTask({ stage: "Plan", state: { type: "NeedsYou", reason: { type: "Replied", first_line: "?" } } }),
  );
  act(() => ref.current!.primaryAction());
  expect(focusSpy).toHaveBeenCalled();
  expect(fakeCommands.tasksAction).not.toHaveBeenCalled();
});
test("growTeam under Swap shows the team in the body and the terminal as a strip", async () => {
  fakeCommands.settingsGet.mockResolvedValue({ ...resolved, team_view_placement: "Swap" });
  const ref = await open(makeTask({ stage: "Build" }));
  await waitFor(() => expect(taskPageStore.store.get().settings["task-1"]?.teamViewPlacement).toBe("Swap"));
  act(() => ref.current!.growTeam());
  expect(screen.getByRole("button", { name: "Expand terminal" })).toBeTruthy();
  fakeCommands.settingsGet.mockResolvedValue(resolved);
});
test("toggleDock flips the dock", async () => {
  const ref = await open(makeTask({ stage: "Plan" }));
  act(() => ref.current!.toggleDock());
  expect(screen.queryByRole("tab", { name: "Memory" })).toBeNull();
});
