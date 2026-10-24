import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { Checkbox } from "./Checkbox";

test("toggles with Space", async () => {
  const on = vi.fn();
  render(<Checkbox label="Flag" onCheckedChange={on} />);
  screen.getByRole("checkbox").focus();
  await setupUser().keyboard(" ");
  expect(on).toHaveBeenCalledWith(true);
});

test("clicking the label toggles", async () => {
  const on = vi.fn();
  render(<Checkbox label="Flag" onCheckedChange={on} />);
  await setupUser().click(screen.getByText("Flag"));
  expect(on).toHaveBeenCalledWith(true);
});

test('indeterminate exposes aria-checked="mixed"', () => {
  render(<Checkbox checked="indeterminate" label="Some" />);
  expect(screen.getByRole("checkbox")).toHaveAttribute("aria-checked", "mixed");
});

test("disabled does not toggle", async () => {
  const on = vi.fn();
  render(<Checkbox label="Flag" disabled onCheckedChange={on} />);
  await setupUser().click(screen.getByText("Flag"));
  expect(on).not.toHaveBeenCalled();
});

test("the description describes the checkbox", () => {
  render(<Checkbox label="Flag" description="Shows a flag on the card" />);
  expect(screen.getByRole("checkbox", { name: "Flag" })).toHaveAccessibleDescription("Shows a flag on the card");
});
