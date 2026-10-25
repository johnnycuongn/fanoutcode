import { render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
import { commands, events, mockBindings } from "@/test/mockBindings";
import { makeTask } from "@/test/fixtures";
import { setupUser } from "@/test/ui";
import { toast } from "@/components/ui";
import type { FileDiff, OverlapRow, PendingDecision, Relation, Task } from "@/lib/bindings";
import { tasksStore } from "@/lib/stores/tasksStore";
import { shellStore } from "@/features/shell";
import { overlapStore } from "../store";
import { OverlapTab } from "./OverlapTab";

const TASK = makeTask({ id: "task-b2", title: "Checkout", stage: "Build" });
const LOGIN = makeTask({ id: "task-c3", title: "Login", stage: "Review" });
const EDIT: PendingDecision = {
  type: "Edit",
  tool: "Edit",
  input: { file_path: "/r/alpha/src/a.ts", old_string: "const a = 1;", new_string: "const a = 2;" },
};

function row(over: Partial<OverlapRow> = {}): OverlapRow {
  return {
    task_id: "task-b2",
    other_task: "task-c3",
    repo_root: "/r/alpha",
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

function diff(before: string, after: string): FileDiff {
  return { repo_root: "/r/alpha", path: "src/a.ts", status: "Modified", before, after, binary: false };
}

function seed(rows: OverlapRow[]): void {
  overlapStore.set({ byTask: { "task-b2": rows }, counts: {} });
}

function renderTab() {
  return render(<OverlapTab task={TASK} projects={[]} />);
}

beforeEach(() => {
  vi.clearAllMocks();
  overlapStore.reset();
  tasksStore.set((s) => ({ ...s, byId: { "task-b2": TASK, "task-c3": LOGIN } }));
  commands.overlapList.mockResolvedValue({ status: "ok", data: [] });
  commands.gitDiff.mockResolvedValue({ status: "ok", data: diff("line\n", "line\nlogin added\n") });
  commands.gitFileRead.mockResolvedValue({ status: "ok", data: "old body\n" });
  commands.overlapPendingInput.mockResolvedValue({ status: "ok", data: EDIT });
  commands.overlapDecide.mockResolvedValue({ status: "ok", data: null });
});

describe("OverlapTab", () => {
  it("lists pending first with the other task's name", () => {
    seed([row({ path: "src/settled.ts" }), row({ path: "src/a.ts", pending: EDIT })]);
    renderTab();
    const needs = screen.getByRole("listbox", { name: "Needs you" });
    const also = screen.getByRole("listbox", { name: "Also touched by others" });
    expect(within(needs).getByRole("option", { name: /src\/a\.ts/ })).toHaveAttribute("aria-selected", "true");
    expect(within(needs).getByText("Login")).toBeInTheDocument();
    expect(within(also).getByRole("option", { name: /src\/settled\.ts/ })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Login is also changing src/a.ts" })).toBeInTheDocument();
  });

  it("renders both sides and the four choices for a pending row", async () => {
    seed([row({ pending: EDIT })]);
    renderTab();
    expect(await screen.findByText("login added")).toBeInTheDocument();
    expect(commands.gitDiff).toHaveBeenCalledWith("task-c3", "/r/alpha", "src/a.ts");
    expect(screen.getByText("Login's changes · +1 −0")).toBeInTheDocument();
    expect(screen.getByText("What Checkout will write")).toBeInTheDocument();
    expect(await screen.findByText("const a = 2;")).toBeInTheDocument();
    expect(screen.getByText("const a = 1;")).toBeInTheDocument();
    expect(commands.overlapPendingInput).toHaveBeenCalledWith("task-b2", "/r/alpha", "src/a.ts");
    const radios = screen.getAllByRole("radio").map((r) => r.getAttribute("aria-labelledby"));
    expect(radios).toHaveLength(4);
    for (const name of [
      "Proceed on top of Login's changes, check the next file",
      "Proceed on top for every file Login touches",
      "Replace Login's changes in this file",
      "Hold",
    ])
      expect(screen.getByRole("radio", { name })).toBeInTheDocument();
    expect(screen.getByText("Login's session is told.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open in Changes" })).toBeInTheDocument();
  });

  it("Continue calls overlapDecide with the selected decision", async () => {
    const rows = [row({ pending: EDIT })];
    seed(rows);
    renderTab();
    const user = setupUser();
    await user.click(screen.getByRole("radio", { name: "Proceed on top for every file Login touches" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() =>
      expect(commands.overlapDecide).toHaveBeenCalledWith("task-b2", "/r/alpha", "src/a.ts", "task-c3", {
        type: "OnTopAll",
      }),
    );
    expect(commands.overlapList).toHaveBeenCalledWith("task-b2");
  });

  it("a settled row shows its relation and no choices", async () => {
    seed([row({ relation: "GrantedOnTop", decided_at: "2026-10-25T08:30:00Z" })]);
    renderTab();
    expect(screen.getAllByText("granted on top").length).toBeGreaterThan(0);
    expect(screen.queryByRole("radiogroup")).toBeNull();
    expect(screen.queryByRole("button", { name: "Continue" })).toBeNull();
    expect(screen.getByText(/Decided/)).toBeInTheDocument();
    expect(await screen.findByText("Checkout's changes · +1 −0")).toBeInTheDocument();
    expect(commands.gitDiff).toHaveBeenCalledWith("task-b2", "/r/alpha", "src/a.ts");
  });

  it("Replace is shown in the danger colour and needs no dialog", async () => {
    seed([row({ pending: EDIT })]);
    renderTab();
    expect(screen.getByText("Replace Login's changes in this file")).toHaveClass("text-bad");
    const user = setupUser();
    await user.click(screen.getByRole("radio", { name: "Replace Login's changes in this file" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    await waitFor(() =>
      expect(commands.overlapDecide).toHaveBeenCalledWith("task-b2", "/r/alpha", "src/a.ts", "task-c3", {
        type: "Replace",
      }),
    );
  });

  it("a command row offers only Proceed and Hold", async () => {
    const cmd: PendingDecision = {
      type: "Command",
      command: "git checkout -- src/a.ts",
      kind: "Discards",
      paths: ["src/a.ts"],
    };
    commands.overlapPendingInput.mockResolvedValue({ status: "ok", data: cmd });
    seed([row({ pending: cmd })]);
    renderTab();
    expect(await screen.findByText("git checkout -- src/a.ts")).toBeInTheDocument();
    expect(screen.getAllByRole("radio").map((r) => r.textContent)).toHaveLength(2);
    expect(screen.getByRole("radio", { name: "Proceed" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Hold" })).toBeInTheDocument();
    await setupUser().click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() =>
      expect(commands.overlapDecide).toHaveBeenCalledWith("task-b2", "/r/alpha", "src/a.ts", "task-c3", {
        type: "OnTopAll",
      }),
    );
  });

  it("a stopped command says what it does to the other task's work", async () => {
    const cmd = (command: string, kind: "Commits" | "Discards"): PendingDecision => ({
      type: "Command",
      command,
      kind,
      paths: ["src/a.ts", "src/b.ts"],
    });
    for (const [pending, line] of [
      [
        cmd("git commit -am wip", "Commits"),
        "Runs `git commit -am wip`: Login's uncommitted changes in src/a.ts, src/b.ts go into this task's commit.",
      ],
      [
        cmd("git reset --hard", "Discards"),
        "Runs `git reset --hard`: Login's uncommitted changes in src/a.ts, src/b.ts are discarded from the working tree.",
      ],
      [
        cmd("git stash -u", "Discards"),
        "Runs `git stash -u`: Login's uncommitted changes in src/a.ts, src/b.ts are stashed and leave the working tree.",
      ],
    ] as const) {
      commands.overlapPendingInput.mockResolvedValue({ status: "ok", data: pending });
      seed([row({ pending })]);
      const { unmount } = renderTab();
      expect(screen.getByText(line)).toBeInTheDocument();
      expect(screen.getByText("Don't run it; you are reminded when Login commits these files.")).toBeInTheDocument();
      unmount();
    }
  });

  it("the told line shows only when the choice tells another session", async () => {
    seed([row({ pending: EDIT })]);
    const { unmount } = renderTab();
    expect(screen.getByText("Login's session is told.")).toBeInTheDocument();
    await setupUser().click(screen.getByRole("radio", { name: "Hold" }));
    expect(screen.queryByText("Login's session is told.")).toBeNull();
    unmount();
    // an unattributed session has no inbox: nobody is told
    seed([row({ pending: EDIT, other_task: null })]);
    renderTab();
    expect(screen.getByRole("radio", { name: /Proceed on top of another session's changes/ })).toBeInTheDocument();
    expect(screen.queryByText(/session is told/)).toBeNull();
  });

  it("a failed decide shows the error", async () => {
    commands.overlapDecide.mockResolvedValue({ status: "error", error: "the claim moved" });
    seed([row({ pending: EDIT })]);
    renderTab();
    await setupUser().click(screen.getByRole("button", { name: "Continue" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("the claim moved");
  });

  it("waits for the tool input and refetches on overlap-changed", async () => {
    commands.overlapPendingInput.mockResolvedValueOnce({ status: "ok", data: null });
    seed([row({ pending: EDIT })]);
    renderTab();
    expect(await screen.findByText("Loading what Checkout will write…")).toBeInTheDocument();
    await waitFor(() => expect(events.overlapChanged.listen).toHaveBeenCalled());
    events.overlapChanged.emitForTest({ task_id: "task-b2", count: 1, pending: 1 });
    expect(await screen.findByText("const a = 2;")).toBeInTheDocument();
  });

  it("a Write is diffed against the current file", async () => {
    const write: PendingDecision = {
      type: "Edit",
      tool: "Write",
      input: { file_path: "src/a.ts", content: "new body\n" },
    };
    commands.overlapPendingInput.mockResolvedValue({ status: "ok", data: write });
    seed([row({ pending: write })]);
    renderTab();
    expect(await screen.findByText("new body")).toBeInTheDocument();
    expect(screen.getByText("old body")).toBeInTheDocument();
    expect(commands.gitFileRead).toHaveBeenCalledWith("task-b2", "/r/alpha", "src/a.ts");
  });

  it("labels relations, holding and an unknown relation, and names another session", () => {
    seed([
      row({ path: "a", relation: "TakenOver" }),
      row({ path: "b", relation: "TakenOverBy" as Relation }),
      row({ path: "c", relation: "PlannedBy", held: true }),
      row({ path: "d", relation: "Brand" as Relation, other_task: null }),
    ]);
    renderTab();
    const list = screen.getByRole("listbox", { name: "Also touched by others" });
    expect(within(list).getByText("taken over")).toBeInTheDocument();
    expect(within(list).getByText("taken over by")).toBeInTheDocument();
    expect(within(list).getByText("holding")).toBeInTheDocument();
    expect(within(list).getByText("Brand")).toBeInTheDocument();
    expect(within(list).getByText("another session")).toBeInTheDocument();
  });

  it("Open in Changes asks the dock for the Changes tab", async () => {
    seed([row({ pending: EDIT })]);
    renderTab();
    await setupUser().click(screen.getByRole("button", { name: "Open in Changes" }));
    expect(shellStore.get().pendingDockTab).toEqual({ taskId: "task-b2", tab: "changes" });
  });

  it("a merge that needs you shows the hand-back pane with Resume, even with no rows", async () => {
    commands.overlapIntegrationContext.mockResolvedValue({ status: "ok", data: [] });
    seed([]);
    const merging = makeTask({
      id: "task-b2",
      stage: "Done",
      state: { type: "NeedsYou", reason: { type: "MergeNeedsYou", repo: "/r/alpha", paths: ["src/a.ts"] } },
    });
    render(<OverlapTab task={merging} projects={[]} />);
    expect(screen.getByRole("region", { name: "Merge needs you" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Resume" })).toBeInTheDocument();
    await waitFor(() => expect(commands.overlapIntegrationContext).toHaveBeenCalledWith("task-b2"));
  });

  it("offers Bring <base> in only on a Done worktree task", () => {
    seed([row()]);
    const done = (over: Partial<Task>) => makeTask({ id: "task-b2", stage: "Done", ...over });
    const { rerender } = render(
      <OverlapTab task={done({ worktree_path: "/wt/b2", base_branch: "main" })} projects={[]} />,
    );
    expect(screen.getByRole("button", { name: "Bring main in" })).toBeInTheDocument();
    rerender(<OverlapTab task={done({ worktree_path: null, base_branch: "main" })} projects={[]} />);
    expect(screen.queryByRole("button", { name: /^Bring .* in$/ })).toBeNull();
    rerender(<OverlapTab task={done({ stage: "Build", worktree_path: "/wt/b2" })} projects={[]} />);
    expect(screen.queryByRole("button", { name: /^Bring .* in$/ })).toBeNull();
  });

  it("Bring base in with no base asks for one first, then integrates", async () => {
    commands.overlapBaseSuggestion.mockResolvedValue({ status: "ok", data: "main" });
    const doneTask = makeTask({ id: "task-b2", stage: "Done", worktree_path: "/wt/b2", base_branch: null });
    commands.overlapSetBase.mockResolvedValue({ status: "ok", data: { ...doneTask, base_branch: "main" } });
    commands.overlapIntegrate.mockResolvedValue({ status: "ok", data: null });
    seed([]);
    render(<OverlapTab task={doneTask} projects={[]} />);
    const user = setupUser();
    await user.click(screen.getByRole("button", { name: "Bring base in" }));
    const input = await screen.findByLabelText("Base branch");
    await waitFor(() => expect(input).toHaveProperty("value", "main"));
    expect(commands.overlapIntegrate).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Bring main in" }));
    await waitFor(() => expect(commands.overlapIntegrate).toHaveBeenCalledWith("task-b2"));
    expect(commands.overlapSetBase).toHaveBeenCalledWith("task-b2", "main");
    expect(tasksStore.get().byId["task-b2"]?.base_branch).toBe("main");
  });

  it("a failed integrate shows the error", async () => {
    commands.overlapIntegrate.mockResolvedValue({ status: "error", error: "Finish the task first." });
    seed([]);
    render(
      <OverlapTab
        task={makeTask({ id: "task-b2", stage: "Done", worktree_path: "/wt/b2", base_branch: "main" })}
        projects={[]}
      />,
    );
    const shown = vi.spyOn(toast, "error");
    await setupUser().click(screen.getByRole("button", { name: "Bring main in" }));
    await waitFor(() => expect(shown).toHaveBeenCalledWith("Finish the task first."));
    shown.mockRestore();
  });
});
