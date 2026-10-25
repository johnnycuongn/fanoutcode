import { render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { RadioGroup } from "./RadioGroup";

const opts = [
  { value: "keep", label: "Keep", description: "Leave it as it is" },
  { value: "drop", label: "Drop", description: "Throw it away", tone: "bad" as const },
  { value: "later", label: "Later", disabled: true },
];

test("a vertical radiogroup whose options carry their label as name and sub-line as description", () => {
  render(<RadioGroup aria-label="Choice" options={opts} value="keep" onValueChange={() => {}} />);
  const group = screen.getByRole("radiogroup", { name: "Choice" });
  expect(group).toHaveAttribute("aria-orientation", "vertical");
  const keep = screen.getByRole("radio", { name: "Keep" });
  expect(keep).toHaveAttribute("aria-checked", "true");
  expect(keep).toHaveAccessibleDescription("Leave it as it is");
});

test("a bad option's label is in the danger colour", () => {
  render(<RadioGroup aria-label="Choice" options={opts} value="keep" onValueChange={() => {}} />);
  expect(screen.getByText("Drop")).toHaveClass("text-bad");
  expect(screen.getByText("Keep")).not.toHaveClass("text-bad");
});

test("clicking an option selects it; a disabled one cannot be chosen", async () => {
  const on = vi.fn();
  render(<RadioGroup aria-label="Choice" options={opts} value="keep" onValueChange={on} />);
  const user = setupUser();
  await user.click(screen.getByText("Throw it away"));
  expect(on).toHaveBeenCalledWith("drop");
  on.mockClear();
  await user.click(screen.getByRole("radio", { name: "Later" }));
  expect(on).not.toHaveBeenCalled();
});

test("ArrowDown moves to the next option", async () => {
  const on = vi.fn();
  render(<RadioGroup aria-label="Choice" options={opts} value="keep" onValueChange={on} />);
  screen.getByRole("radio", { name: "Keep" }).focus();
  const user = setupUser();
  await user.keyboard("{ArrowDown>}");
  await waitFor(() => expect(on).toHaveBeenCalledWith("drop"));
  await user.keyboard("{/ArrowDown}");
});
