import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
vi.mock("../ipc");
import { resetBoardStore } from "../store";
import { makeTask } from "../testing/fixtures";
import { mockIpc } from "../testing/mockIpc";
import { JiraKeyDialog } from "./JiraKeyDialog";

describe("JiraKeyDialog", () => {
  it("rejects a malformed key and saves a valid one through tasks_update", async () => {
    resetBoardStore();
    const m = mockIpc();
    const onClose = vi.fn();
    render(<JiraKeyDialog task={makeTask({ id: "t-1" })} open onClose={onClose} />);
    await userEvent.type(screen.getByLabelText("Issue key"), "nope");
    await userEvent.click(screen.getByRole("button", { name: "Link" }));
    expect(screen.getByText("Use the form PROJ-123")).toBeInTheDocument();
    expect(m.ipc.tasksUpdate).not.toHaveBeenCalled();
    await userEvent.clear(screen.getByLabelText("Issue key"));
    await userEvent.type(screen.getByLabelText("Issue key"), "TC-12");
    await userEvent.click(screen.getByRole("button", { name: "Link" }));
    expect(m.ipc.tasksUpdate).toHaveBeenCalledWith("t-1", { jira_key: "TC-12" });
    expect(onClose).toHaveBeenCalled();
  });
  it("upper-cases a lower-case key", async () => {
    resetBoardStore();
    const m = mockIpc();
    render(<JiraKeyDialog task={makeTask({ id: "t-1" })} open onClose={vi.fn()} />);
    await userEvent.type(screen.getByLabelText("Issue key"), "tc-7{Enter}");
    expect(m.ipc.tasksUpdate).toHaveBeenCalledWith("t-1", { jira_key: "TC-7" });
  });
  it("offers Unlink for a linked task", async () => {
    resetBoardStore();
    const m = mockIpc();
    render(<JiraKeyDialog task={makeTask({ id: "t-1", jira_key: "TC-1" })} open onClose={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: "Unlink" }));
    expect(m.ipc.tasksUpdate).toHaveBeenCalledWith("t-1", { jira_key: null });
  });
});
