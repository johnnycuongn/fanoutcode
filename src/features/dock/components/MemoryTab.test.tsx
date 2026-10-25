import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
vi.mock("@/lib/bindings", async () => ({ events: (await import("@/test/taskPageFixtures")).fakeEvents }));
import { makeTask } from "@/test/taskPageFixtures";
import { MemoryTab } from "./MemoryTab";

const ipc = () => ({
  memoryRead: vi.fn(async (_t: string, f: string) => `contents of ${f}`),
  memoryWrite: vi.fn(async () => null),
});
test("opens on the stage's file and switches files", async () => {
  const i = ipc();
  render(<MemoryTab task={makeTask({ stage: "Build" })} ipc={i} />);
  await screen.findByText("contents of plan.md");
  fireEvent.click(screen.getByRole("radio", { name: "Intent" }));
  await screen.findByText("contents of intent.md");
});
test("the edit toggle writes the buffer through memory_write when switched off", async () => {
  const i = ipc();
  render(<MemoryTab task={makeTask({ stage: "Plan" })} ipc={i} />);
  await screen.findByText("contents of plan.md");
  fireEvent.click(screen.getByRole("switch", { name: "Edit" }));
  fireEvent.change(screen.getByRole("textbox", { name: "plan.md" }), { target: { value: "# plan.md\n- [ ] 1. x" } });
  fireEvent.click(screen.getByRole("switch", { name: "Edit" }));
  await waitFor(() => expect(i.memoryWrite).toHaveBeenCalledWith("task-1", "plan.md", "# plan.md\n- [ ] 1. x"));
});
test("index.md has no edit toggle", async () => {
  render(<MemoryTab task={makeTask({ stage: "Done" })} ipc={ipc()} />);
  await screen.findByText("contents of index.md");
  expect(screen.queryByRole("switch", { name: "Edit" })).toBeNull();
  expect(screen.getByText("Written by Fanout Code")).toBeTruthy();
});
test("a memory_read failure shows the message verbatim and no editor", async () => {
  const i = {
    ...ipc(),
    memoryRead: vi.fn(async () => {
      throw "Memory folder missing: /m/2026-10-24-a";
    }),
  };
  render(<MemoryTab task={makeTask()} ipc={i} />);
  await screen.findByText("Memory folder missing: /m/2026-10-24-a");
  expect(screen.queryByRole("switch", { name: "Edit" })).toBeNull();
});
test("a stage change while editing keeps the open file and its buffer", async () => {
  const i = ipc();
  const { rerender } = render(<MemoryTab task={makeTask({ stage: "Plan" })} ipc={i} />);
  await screen.findByText("contents of plan.md");
  fireEvent.click(screen.getByRole("switch", { name: "Edit" }));
  fireEvent.change(screen.getByRole("textbox", { name: "plan.md" }), { target: { value: "mine" } });
  rerender(<MemoryTab task={makeTask({ stage: "Build" })} ipc={i} />);
  rerender(<MemoryTab task={makeTask({ stage: "Review" })} ipc={i} />);
  expect((screen.getByRole("textbox", { name: "plan.md" }) as HTMLTextAreaElement).value).toBe("mine");
});
