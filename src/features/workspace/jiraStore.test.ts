import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands } from "@/test/mockBindings";
import type { Issue } from "@/lib/bindings";
import { jiraStore } from "./jiraStore";

const ok = <T>(data: T) => ({ status: "ok" as const, data });
const issue = (key: string, summary: string): Issue => ({
  key,
  summary,
  status: "To Do",
  assignee: null,
  issue_type: "Story",
  description: "",
  url: `https://acme.atlassian.net/browse/${key}`,
  updated: "2026-10-24T05:12:00Z",
});
const flush = () => new Promise((r) => setTimeout(r, 0));

describe("jiraStore", () => {
  beforeEach(() => {
    jiraStore.reset();
    vi.clearAllMocks();
  });
  afterEach(() => vi.useRealTimers());

  it("link calls the command and never keeps the token in state", async () => {
    commands.jiraLink.mockResolvedValue(ok(null));
    commands.jiraStatus.mockResolvedValue(
      ok({ site: { site_url: "https://acme.atlassian.net", email: "me@acme.com" }, inherited: false }),
    );
    await jiraStore.link({ project: "proj-1" }, "https://acme.atlassian.net", "me@acme.com", "secret-token");
    expect(commands.jiraLink).toHaveBeenCalledWith(
      "proj-1",
      "https://acme.atlassian.net",
      "me@acme.com",
      "secret-token",
    );
    expect(JSON.stringify(jiraStore.get())).not.toContain("secret-token");
    expect(jiraStore.get().status["proj-1"].site?.email).toBe("me@acme.com");
  });

  it("workspace scope uses jiraLinkWorkspace", async () => {
    commands.jiraLinkWorkspace.mockResolvedValue(ok(null));
    await jiraStore.link({ workspace: "ws-1" }, "https://acme.atlassian.net", "me@acme.com", "t");
    expect(commands.jiraLinkWorkspace).toHaveBeenCalledWith("ws-1", "https://acme.atlassian.net", "me@acme.com", "t");
  });

  it("issue returns cached value then refreshes", async () => {
    jiraStore.set((s) => ({ ...s, issues: { "proj-1:PROJ-1": issue("PROJ-1", "old") } }));
    commands.jiraIssue.mockResolvedValue(ok(issue("PROJ-1", "new")));
    expect((await jiraStore.issue("proj-1", "PROJ-1")).summary).toBe("old");
    await flush();
    expect(jiraStore.get().issues["proj-1:PROJ-1"].summary).toBe("new");
  });

  it("search debounces and stores results", async () => {
    vi.useFakeTimers();
    commands.jiraSearch.mockResolvedValue(ok([issue("PROJ-2", "x")]));
    void jiraStore.search("proj-1", "lo");
    void jiraStore.search("proj-1", "login");
    await vi.advanceTimersByTimeAsync(300);
    expect(commands.jiraSearch).toHaveBeenCalledTimes(1);
    expect(commands.jiraSearch).toHaveBeenCalledWith("proj-1", "login");
    expect(jiraStore.get().search.results.map((i) => i.key)).toEqual(["PROJ-2"]);
  });

  it("createTask forwards the workspace id and returns the task", async () => {
    commands.jiraCreateTask.mockResolvedValue(ok({ id: "task-1" }));
    expect((await jiraStore.createTask("proj-1", "PROJ-1", "ws-1")).id).toBe("task-1");
    expect(commands.jiraCreateTask).toHaveBeenCalledWith("proj-1", "PROJ-1", "ws-1");
  });

  it("an error is kept and rethrown", async () => {
    commands.jiraComment.mockResolvedValue({ status: "error", error: "Jira: HTTP 500" });
    await expect(jiraStore.comment("proj-1", "PROJ-1", "x")).rejects.toThrow("Jira: HTTP 500");
    expect(jiraStore.get().error).toBe("Jira: HTTP 500");
  });
});
