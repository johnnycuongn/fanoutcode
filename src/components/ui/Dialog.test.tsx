import { render, screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { mockMatchMedia, setupUser } from "@/test/ui";
import { Button } from "./Button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "./Dialog";

function Fixture() {
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button>Archive</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Archive task</DialogTitle>
          <DialogDescription>The agent is still working.</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <DialogClose asChild>
            <Button>Cancel</Button>
          </DialogClose>
          <Button variant="danger">Archive</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

test("opens from its trigger, moves focus inside, closes on Escape and returns focus", async () => {
  render(<Fixture />);
  const user = setupUser();
  const t = screen.getByRole("button", { name: "Archive" });
  await user.click(t);
  const dialog = await screen.findByRole("dialog", { name: "Archive task" });
  expect(dialog).toContainElement(document.activeElement as HTMLElement);
  await user.keyboard("{Escape}");
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  expect(t).toHaveFocus();
});

test("the close button closes it", async () => {
  render(<Fixture />);
  const user = setupUser();
  await user.click(screen.getByRole("button", { name: "Archive" }));
  await user.click(await screen.findByRole("button", { name: "Close" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
});

test("is described by its description", async () => {
  render(<Fixture />);
  await setupUser().click(screen.getByRole("button", { name: "Archive" }));
  expect(await screen.findByRole("dialog")).toHaveAccessibleDescription("The agent is still working.");
});

test("full motion zooms in over 200 ms", async () => {
  render(<Fixture />);
  await setupUser().click(screen.getByRole("button", { name: "Archive" }));
  const d = await screen.findByRole("dialog");
  expect(d).toHaveAttribute("data-motion", "full");
  expect(d).toHaveClass("duration-200", "data-[state=open]:zoom-in-95");
});

test("reduced motion fades over 150 ms with no zoom", async () => {
  mockMatchMedia(true);
  render(<Fixture />);
  await setupUser().click(screen.getByRole("button", { name: "Archive" }));
  const d = await screen.findByRole("dialog");
  expect(d).toHaveAttribute("data-motion", "reduced");
  expect(d).toHaveClass("duration-150");
  expect(d).not.toHaveClass("data-[state=open]:zoom-in-95");
});

test("hideClose removes the close button", async () => {
  render(
    <Dialog defaultOpen>
      <DialogContent hideClose>
        <DialogTitle>No close</DialogTitle>
        <DialogDescription>Body</DialogDescription>
      </DialogContent>
    </Dialog>,
  );
  await screen.findByRole("dialog");
  expect(screen.queryByRole("button", { name: "Close" })).toBeNull();
});

test("nested dialog: Escape closes only the inner dialog and focus returns to its trigger", async () => {
  render(
    <Dialog defaultOpen>
      <DialogContent>
        <DialogTitle>Outer</DialogTitle>
        <Dialog>
          <DialogTrigger asChild>
            <Button>More</Button>
          </DialogTrigger>
          <DialogContent>
            <DialogTitle>Inner</DialogTitle>
          </DialogContent>
        </Dialog>
      </DialogContent>
    </Dialog>,
  );
  const user = setupUser();
  await user.click(await screen.findByRole("button", { name: "More" }));
  await screen.findByRole("dialog", { name: "Inner" });
  await user.keyboard("{Escape}");
  await waitFor(() => expect(screen.queryByRole("dialog", { name: "Inner" })).not.toBeInTheDocument());
  expect(screen.getByRole("dialog", { name: "Outer" })).toBeInTheDocument();
  await waitFor(() => expect(screen.getByRole("button", { name: "More" })).toHaveFocus());
});
