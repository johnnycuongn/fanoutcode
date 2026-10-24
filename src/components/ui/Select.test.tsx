import { render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { Field } from "./Field";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./Select";

function Fixture({ onValueChange, value }: { onValueChange?: (v: string) => void; value?: string }) {
  return (
    <Select value={value} onValueChange={onValueChange}>
      <SelectTrigger aria-label="Priority">
        <SelectValue placeholder="Priority" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="none">None</SelectItem>
        <SelectItem value="low">Low</SelectItem>
        <SelectItem value="medium">Medium</SelectItem>
        <SelectItem value="high">High</SelectItem>
      </SelectContent>
    </Select>
  );
}

test("opens with ArrowDown, moves the highlight and selects with Enter", async () => {
  const on = vi.fn();
  render(<Fixture onValueChange={on} />);
  const user = setupUser();
  screen.getByRole("combobox", { name: "Priority" }).focus();
  await user.keyboard("{ArrowDown}");
  expect(await screen.findByRole("listbox")).toBeInTheDocument();
  await user.keyboard("{ArrowDown}{Enter}");
  expect(on).toHaveBeenCalledWith("low");
});

test("Escape closes the list and returns focus to the trigger", async () => {
  render(<Fixture />);
  const user = setupUser();
  const t = screen.getByRole("combobox");
  t.focus();
  await user.keyboard("{Enter}");
  await screen.findByRole("listbox");
  await user.keyboard("{Escape}");
  await waitFor(() => expect(screen.queryByRole("listbox")).not.toBeInTheDocument());
  expect(t).toHaveFocus();
});

test("shows the selected label in the trigger", () => {
  render(<Fixture value="high" />);
  expect(screen.getByRole("combobox")).toHaveTextContent("High");
});

test("picks up Field ids and invalid state", () => {
  render(
    <Field label="Agent" error="Pick one">
      <Fixture />
    </Field>,
  );
  const t = screen.getByRole("combobox", { name: "Priority" });
  expect(t).toHaveAttribute("aria-invalid", "true");
  expect(t).toHaveAttribute("id", expect.stringMatching(/./));
});
