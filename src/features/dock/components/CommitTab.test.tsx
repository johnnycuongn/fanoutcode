import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
vi.mock("@/lib/bindings", async () => ({ events: (await import("@/test/taskPageFixtures")).fakeEvents }));
import { makeSettings, makeTask } from "@/test/taskPageFixtures";
import { drafts } from "../testing/fakes";
import { createCommitStore, type CommitIpc } from "../commitStore";
import { CommitTab } from "./CommitTab";

const ipc = () => ({ gitDrafts: vi.fn(async () => drafts), tasksAction: vi.fn(async () => {}) });
const clip = () => ({ write: vi.fn(async () => {}) });
test("renders drafts grouped by repo with Copy, which copies and records CopyCommit", async () => {
  const i = ipc();
  const c = clip();
  const s = createCommitStore(i as unknown as CommitIpc, c);
  render(<CommitTab task={makeTask({ stage: "Commit" })} settings={makeSettings()} store={s} />);
  await screen.findByText("api");
  fireEvent.click(screen.getAllByRole("button", { name: "Copy" })[0]);
  await waitFor(() => expect(c.write).toHaveBeenCalledWith("feat(auth): constant-time token compare\n\nCloses TC-12."));
  expect(i.tasksAction).toHaveBeenCalledWith("task-1", { type: "CopyCommit" });
  expect(screen.queryByRole("button", { name: "Commit" })).toBeNull();
});
test("the Commit button exists only with may_commit on and confirms before sending", async () => {
  const i = ipc();
  const s = createCommitStore(i as unknown as CommitIpc, clip());
  render(<CommitTab task={makeTask({ stage: "Commit" })} settings={makeSettings({ mayCommit: true })} store={s} />);
  fireEvent.click(await screen.findByRole("button", { name: "Commit" }));
  fireEvent.click(await screen.findByRole("button", { name: "Commit 3 files in 2 repos" }));
  await waitFor(() => expect(i.tasksAction).toHaveBeenCalledWith("task-1", { type: "Commit" }));
});
