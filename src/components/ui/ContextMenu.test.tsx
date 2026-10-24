import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from "./ContextMenu";

function Fixture({ onOpen }: { onOpen?: () => void }) {
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div>Card</div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onSelect={onOpen}>Open</ContextMenuItem>
        <ContextMenuItem danger>Archive</ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}

test("opens on right-click", async () => {
  render(<Fixture />);
  fireEvent.contextMenu(screen.getByText("Card"));
  expect(await screen.findByRole("menu")).toBeInTheDocument();
});

test("ArrowDown and Enter select an item", async () => {
  const onOpen = vi.fn();
  render(<Fixture onOpen={onOpen} />);
  fireEvent.contextMenu(screen.getByText("Card"));
  await screen.findByRole("menu");
  await setupUser().keyboard("{ArrowDown}{Enter}");
  expect(onOpen).toHaveBeenCalledTimes(1);
});

test("Escape closes the menu", async () => {
  render(<Fixture />);
  fireEvent.contextMenu(screen.getByText("Card"));
  await screen.findByRole("menu");
  await setupUser().keyboard("{Escape}");
  await waitFor(() => expect(screen.queryByRole("menu")).not.toBeInTheDocument());
});
