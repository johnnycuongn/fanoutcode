import { render, screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { setupUser } from "@/test/ui";
import { Button } from "./Button";
import { Popover, PopoverContent, PopoverTrigger } from "./Popover";

test("opens on trigger click and closes on Escape returning focus", async () => {
  const user = setupUser();
  render(
    <Popover>
      <PopoverTrigger asChild>
        <Button>Filters</Button>
      </PopoverTrigger>
      <PopoverContent>Body</PopoverContent>
    </Popover>,
  );
  await user.click(screen.getByRole("button", { name: "Filters" }));
  expect(await screen.findByText("Body")).toBeVisible();
  await user.keyboard("{Escape}");
  await waitFor(() => expect(screen.queryByText("Body")).not.toBeInTheDocument());
  expect(screen.getByRole("button", { name: "Filters" })).toHaveFocus();
});

test('width="trigger" sizes the content to the trigger', async () => {
  render(
    <Popover defaultOpen>
      <PopoverTrigger>t</PopoverTrigger>
      <PopoverContent width="trigger">Body</PopoverContent>
    </Popover>,
  );
  expect((await screen.findByText("Body")).closest("[data-state]")).toHaveClass(
    "w-[var(--radix-popover-trigger-width)]",
  );
});

test("data-motion follows reduced motion", async () => {
  render(
    <Popover defaultOpen>
      <PopoverTrigger>t</PopoverTrigger>
      <PopoverContent>Body</PopoverContent>
    </Popover>,
  );
  expect((await screen.findByText("Body")).closest("[data-state]")).toHaveAttribute("data-motion", "full");
});
