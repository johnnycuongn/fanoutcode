import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { makeTask } from "@/test/taskPageFixtures";
import { createSpecStore } from "../store";
import type { SpecIpc } from "../types";
import { SpecStagePage } from "./SpecStagePage";

function fakeIpc(): SpecIpc {
  return {
    memoryRead: vi.fn(async () => "# Spec"),
    memoryWrite: vi.fn(async () => {}),
    settingsGet: vi.fn(async () => ({ livespec_level: "Simple" })),
    settingsSet: vi.fn(async () => {}),
    tasksAction: vi.fn(async () => {}),
    livespecAnalyze: vi.fn(async () => {}),
    livespecCancel: vi.fn(async () => {}),
    sessionsOpen: vi.fn(async () => "sess-9"),
  } as unknown as SpecIpc;
}

test("the terminal is a strip by default and expands on the toggle", async () => {
  const s = createSpecStore(fakeIpc());
  const t = makeTask({ stage: "Spec" });
  await s.open(t);
  render(<SpecStagePage task={t} terminal={<div data-testid="term" />} store={s} />);
  expect(screen.queryByTestId("term")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Expand terminal" }));
  expect(screen.getByTestId("term")).toBeTruthy();
  expect(s.store.get().byTask["task-1"].terminalExpanded).toBe(true);
});
test("without a session the strip offers to start one", async () => {
  const ipc = fakeIpc();
  const s = createSpecStore(ipc);
  const t = makeTask({ stage: "Spec", primary_session: null });
  await s.open(t);
  render(<SpecStagePage task={t} terminal={null} store={s} />);
  fireEvent.click(screen.getByRole("button", { name: "Start session" }));
  expect(ipc.sessionsOpen).toHaveBeenCalledWith("task-1");
});
test("the LiveSpec switch shows the remembered level and writes a change", async () => {
  const ipc = fakeIpc();
  const s = createSpecStore(ipc);
  const t = makeTask({ stage: "Spec" });
  await s.open(t);
  render(<SpecStagePage task={t} terminal={null} store={s} />);
  expect(screen.getByRole("radio", { name: "Simple", checked: true })).toBeTruthy();
  fireEvent.click(screen.getByRole("radio", { name: "Heavy" }));
  await waitFor(() => expect(ipc.settingsSet).toHaveBeenCalledWith(expect.anything(), "livespec_level", "Heavy"));
});
test("a missing memory folder shows the message and a read-only editor", async () => {
  const ipc = { ...fakeIpc(), memoryRead: vi.fn(async () => Promise.reject("Memory folder missing: /m")) };
  const s = createSpecStore(ipc as unknown as SpecIpc);
  const t = makeTask({ stage: "Spec" });
  await s.open(t);
  render(<SpecStagePage task={t} terminal={null} store={s} />);
  expect(screen.getByRole("alert").textContent).toBe("Memory folder missing: /m");
  expect(screen.getByRole("textbox", { name: "Spec" }).hasAttribute("readonly")).toBe(true);
});
