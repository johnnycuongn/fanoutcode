import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
vi.mock("../ipc");
import { resetBoardStore, upsertTask } from "../store";
import { makeTask } from "../testing/fixtures";
import { mockIpc } from "../testing/mockIpc";
import { RenameInline } from "./RenameInline";

describe("RenameInline", () => {
  it("Enter commits the new title through tasks_rename and Escape cancels", async () => {
    resetBoardStore();
    const m = mockIpc();
    const onDone = vi.fn();
    upsertTask(makeTask({ id: "t-1", title: "Old" }));
    upsertTask(makeTask({ id: "t-2", title: "Keep" }));
    render(<RenameInline task={makeTask({ id: "t-1", title: "Old" })} onDone={onDone} />);
    const input = screen.getByRole("textbox");
    expect(input).toHaveFocus();
    expect(input).toHaveValue("Old");
    await userEvent.clear(input);
    await userEvent.type(input, "New{Enter}");
    expect(m.ipc.tasksRename).toHaveBeenCalledWith("t-1", "New");
    expect(onDone).toHaveBeenCalled();
    render(<RenameInline task={makeTask({ id: "t-2", title: "Keep" })} onDone={onDone} />);
    await userEvent.type(screen.getAllByRole("textbox").at(-1)!, "x{Escape}");
    expect(m.ipc.tasksRename).not.toHaveBeenCalledWith("t-2", expect.anything());
  });
  it("commits once when Enter is followed by the blur that unmounting causes", async () => {
    resetBoardStore();
    const m = mockIpc();
    upsertTask(makeTask({ id: "t-1", title: "Old" }));
    render(<RenameInline task={makeTask({ id: "t-1", title: "Old" })} onDone={vi.fn()} />);
    await userEvent.type(screen.getByRole("textbox"), "er{Enter}");
    await userEvent.tab();
    expect(m.ipc.tasksRename).toHaveBeenCalledTimes(1);
  });
  it("blur commits", async () => {
    resetBoardStore();
    const m = mockIpc();
    upsertTask(makeTask({ id: "t-1", title: "Old" }));
    render(<RenameInline task={makeTask({ id: "t-1", title: "Old" })} onDone={vi.fn()} />);
    await userEvent.type(screen.getByRole("textbox"), "er");
    await userEvent.tab();
    expect(m.ipc.tasksRename).toHaveBeenCalledWith("t-1", "Older");
  });
});
