import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("../ipc");
import { boardStore, resetBoardStore } from "../store";
import { cardMenuItems, type MenuCtx } from "../menu";
import { makeTask } from "../testing/fixtures";
import { mockIpc } from "../testing/mockIpc";
import { CardMenu, CardMenuButton } from "./CardMenu";

const ctx: MenuCtx = {
  windows: [],
  currentWindowId: "w-1",
  onOpen: vi.fn(),
  confirm: vi.fn(async () => true),
  openJira: vi.fn(),
};

describe("CardMenu", () => {
  let m: ReturnType<typeof mockIpc>;
  beforeEach(() => {
    resetBoardStore();
    m = mockIpc();
  });
  it("the … button opens the menu and Rename starts renaming", async () => {
    const task = makeTask({ id: "t-1" });
    render(<CardMenuButton items={cardMenuItems(task, ctx)} />);
    await userEvent.click(screen.getByRole("button", { name: "Card actions" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: /Rename/ }));
    expect(boardStore.get().renamingTaskId).toBe("t-1");
  });
  it("a priority submenu item updates the task", async () => {
    const task = makeTask({ id: "t-1" });
    render(<CardMenuButton items={cardMenuItems(task, ctx)} />);
    await userEvent.click(screen.getByRole("button", { name: "Card actions" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: /Priority/ }));
    await userEvent.click(await screen.findByRole("menuitemradio", { name: "High" }));
    expect(m.ipc.tasksUpdate).toHaveBeenCalledWith("t-1", { priority: "High" });
  });
  it("shows why the worktree item is disabled", async () => {
    const task = makeTask({ id: "t-1", primary_session: "s-1" });
    render(<CardMenuButton items={cardMenuItems(task, ctx)} />);
    await userEvent.click(screen.getByRole("button", { name: "Card actions" }));
    expect(await screen.findByText(/Set when the session started/)).toBeInTheDocument();
  });
  it("right-clicking the wrapped card opens the same menu", async () => {
    const task = makeTask({ id: "t-1" });
    render(
      <CardMenu items={cardMenuItems(task, ctx)}>
        <p>card body</p>
      </CardMenu>,
    );
    await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByText("card body") });
    expect(await screen.findByRole("menuitem", { name: /Archive/ })).toBeInTheDocument();
  });
});
