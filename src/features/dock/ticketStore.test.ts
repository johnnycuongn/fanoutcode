import { expect, test, vi, type Mock } from "vitest";
import { makeTask } from "@/test/taskPageFixtures";
import type { Issue } from "./jira";
import { createTicketStore, type TicketIpc } from "./ticketStore";

const issue: Issue = {
  key: "TC-12",
  summary: "Add dock",
  status: "In Progress",
  assignee: "Johnny",
  url: "https://x.atlassian.net/browse/TC-12",
};
const fakeIpc = (): TicketIpc =>
  ({
    jiraIssue: vi.fn(async () => issue),
    jiraComment: vi.fn(async () => {}),
    jiraSearch: vi.fn(async () => [issue]),
    tasksUpdate: vi.fn(async () => {}),
  }) as unknown as TicketIpc;
test("load fetches the linked issue and skips when the task has no key", async () => {
  const ipc = fakeIpc();
  const s = createTicketStore(ipc);
  await s.load(makeTask({ jira_key: null }));
  expect(ipc.jiraIssue).not.toHaveBeenCalled();
  await s.load(makeTask({ jira_key: "TC-12" }));
  expect(ipc.jiraIssue).toHaveBeenCalledWith("proj-1", "TC-12");
  expect(s.store.get()["task-1"].issue).toEqual(issue);
});
test("a Precondition error is shown verbatim", async () => {
  const ipc = fakeIpc();
  (ipc.jiraIssue as Mock).mockRejectedValueOnce("Link Jira in project settings first.");
  const s = createTicketStore(ipc);
  await s.load(makeTask({ jira_key: "TC-12" }));
  expect(s.store.get()["task-1"].error).toBe("Link Jira in project settings first.");
});
test("comment posts and reports success; link updates the task and loads the issue", async () => {
  const ipc = fakeIpc();
  const s = createTicketStore(ipc);
  const t = makeTask({ jira_key: "TC-12" });
  await expect(s.comment(t, "Looks good")).resolves.toBe(true);
  expect(ipc.jiraComment).toHaveBeenCalledWith("proj-1", "TC-12", "Looks good");
  await s.link(makeTask({ jira_key: null }), "TC-12");
  expect(ipc.tasksUpdate).toHaveBeenCalledWith("task-1", { jira_key: "TC-12" });
  expect(ipc.jiraIssue).toHaveBeenCalledWith("proj-1", "TC-12");
});
test("search fills results", async () => {
  const ipc = fakeIpc();
  const s = createTicketStore(ipc);
  await s.search(makeTask(), "dock");
  expect(ipc.jiraSearch).toHaveBeenCalledWith("proj-1", "dock");
  expect(s.store.get()["task-1"].results).toEqual([issue]);
});
