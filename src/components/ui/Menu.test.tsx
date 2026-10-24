import { render, screen, waitFor } from "@testing-library/react";
import { Archive } from "lucide-react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { IconButton } from "./IconButton";
import { Menu, MenuContent, MenuItem, MenuSeparator, MenuTrigger } from "./Menu";

function Fixture({
  onRename,
  onToggle,
  onArchive,
}: {
  onRename?: () => void;
  onToggle?: () => void;
  onArchive?: () => void;
}) {
  return (
    <Menu>
      <MenuTrigger asChild>
        <IconButton label="Card menu" icon={<Archive />} />
      </MenuTrigger>
      <MenuContent>
        <MenuItem shortcut="cmd+e" onSelect={onRename}>
          Rename
        </MenuItem>
        <MenuItem disabled hint="Session started" onSelect={onToggle}>
          Toggle worktree
        </MenuItem>
        <MenuSeparator />
        <MenuItem danger shortcut="cmd+shift+a" onSelect={onArchive}>
          Archive
        </MenuItem>
      </MenuContent>
    </Menu>
  );
}

async function openWithEnter() {
  screen.getByRole("button", { name: "Card menu" }).focus();
  const user = setupUser();
  await user.keyboard("{Enter}");
  await screen.findByRole("menu");
  return user;
}

test("Enter on the trigger opens the menu and focuses the first item", async () => {
  render(<Fixture />);
  await openWithEnter();
  expect(screen.getByRole("menuitem", { name: /Rename/ })).toHaveFocus();
});

test("ArrowDown moves to the next enabled item and Enter selects it", async () => {
  const onArchive = vi.fn();
  render(<Fixture onArchive={onArchive} />);
  const user = await openWithEnter();
  await user.keyboard("{ArrowDown}{Enter}");
  expect(onArchive).toHaveBeenCalledTimes(1);
  await waitFor(() => expect(screen.queryByRole("menu")).not.toBeInTheDocument());
});

test("Escape closes and returns focus to the trigger", async () => {
  render(<Fixture />);
  const user = await openWithEnter();
  await user.keyboard("{Escape}");
  await waitFor(() => expect(screen.queryByRole("menu")).not.toBeInTheDocument());
  expect(screen.getByRole("button", { name: "Card menu" })).toHaveFocus();
});

test("renders shortcut glyphs and hints", async () => {
  render(<Fixture />);
  await openWithEnter();
  expect(screen.getByText("⌘⇧A")).toBeInTheDocument();
  expect(screen.getByText("Session started")).toBeInTheDocument();
});

test("a disabled item is aria-disabled and does not fire onSelect", async () => {
  const onToggle = vi.fn();
  render(<Fixture onToggle={onToggle} />);
  const user = await openWithEnter();
  const item = screen.getByRole("menuitem", { name: /Toggle worktree/ });
  expect(item).toHaveAttribute("aria-disabled", "true");
  await user.click(item);
  expect(onToggle).not.toHaveBeenCalled();
});

test("danger items use the bad tone", async () => {
  render(<Fixture />);
  await openWithEnter();
  expect(screen.getByRole("menuitem", { name: /Archive/ })).toHaveClass("text-bad");
});
