import { render, screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { mockMatchMedia, setupUser } from "@/test/ui";
import { Button } from "./Button";
import { Drawer, DrawerContent, DrawerDescription, DrawerHeader, DrawerTitle, DrawerTrigger } from "./Drawer";

function Fixture({ side }: { side?: "bottom" | "right" }) {
  return (
    <Drawer side={side}>
      <DrawerTrigger asChild>
        <Button>Costs</Button>
      </DrawerTrigger>
      <DrawerContent>
        <DrawerHeader>
          <DrawerTitle>Costs</DrawerTitle>
          <DrawerDescription>Spend by task.</DrawerDescription>
        </DrawerHeader>
      </DrawerContent>
    </Drawer>
  );
}

test("opens from its trigger and closes on Escape", async () => {
  render(<Fixture />);
  const user = setupUser();
  await user.click(screen.getByRole("button", { name: "Costs" }));
  expect(await screen.findByRole("dialog", { name: "Costs" })).toBeInTheDocument();
  await user.keyboard("{Escape}");
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
});

test('side="right" docks to the right edge', async () => {
  render(<Fixture side="right" />);
  await setupUser().click(screen.getByRole("button", { name: "Costs" }));
  expect(await screen.findByRole("dialog")).toHaveClass("right-0", "inset-y-0");
});

test("bottom drawers render a handle", async () => {
  render(<Fixture />);
  await setupUser().click(screen.getByRole("button", { name: "Costs" }));
  expect((await screen.findByRole("dialog")).querySelector("[data-vaul-handle]")).not.toBeNull();
});

test("reduced motion marks the content so the 150 ms override applies", async () => {
  mockMatchMedia(true);
  render(<Fixture />);
  await setupUser().click(screen.getByRole("button", { name: "Costs" }));
  expect(await screen.findByRole("dialog")).toHaveAttribute("data-motion", "reduced");
});

test("full motion marks the content full", async () => {
  render(<Fixture />);
  await setupUser().click(screen.getByRole("button", { name: "Costs" }));
  expect(await screen.findByRole("dialog")).toHaveAttribute("data-motion", "full");
});
