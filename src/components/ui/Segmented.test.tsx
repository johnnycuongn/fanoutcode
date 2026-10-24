import { render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { Segmented } from "./Segmented";

const opts = [
  { value: "off", label: "Off" },
  { value: "simple", label: "Simple" },
  { value: "heavy", label: "Heavy" },
] as const;

test("renders a radiogroup with the selected option checked", () => {
  render(<Segmented aria-label="LiveSpec" options={opts} value="simple" onValueChange={() => {}} />);
  expect(screen.getByRole("radiogroup", { name: "LiveSpec" })).toBeInTheDocument();
  expect(screen.getByRole("radio", { name: "Simple" })).toHaveAttribute("aria-checked", "true");
});

// Radix selects on arrow focus only while the arrow key is still down, and user-event releases it instantly, so
// these tests hold the key (as a person does) while the roving focus settles.
test("ArrowRight selects the next option", async () => {
  const on = vi.fn();
  render(<Segmented aria-label="L" options={opts} value="off" onValueChange={on} />);
  screen.getByRole("radio", { name: "Off" }).focus();
  const user = setupUser();
  await user.keyboard("{ArrowRight>}");
  await waitFor(() => expect(on).toHaveBeenCalledWith("simple"));
  await user.keyboard("{/ArrowRight}");
});

test("ArrowLeft wraps from the first to the last option", async () => {
  const on = vi.fn();
  render(<Segmented aria-label="L" options={opts} value="off" onValueChange={on} />);
  screen.getByRole("radio", { name: "Off" }).focus();
  const user = setupUser();
  await user.keyboard("{ArrowLeft>}");
  await waitFor(() => expect(on).toHaveBeenCalledWith("heavy"));
  await user.keyboard("{/ArrowLeft}");
});

test("clicking an option selects it", async () => {
  const on = vi.fn();
  render(<Segmented aria-label="L" options={opts} value="off" onValueChange={on} />);
  await setupUser().click(screen.getByRole("radio", { name: "Heavy" }));
  expect(on).toHaveBeenCalledWith("heavy");
});

test("Tab lands on the selected option and the group is one tab stop", async () => {
  render(
    <>
      <Segmented aria-label="L" options={opts} value="simple" onValueChange={() => {}} />
      <button type="button">after</button>
    </>,
  );
  const user = setupUser();
  await user.tab();
  expect(screen.getByRole("radio", { name: "Simple" })).toHaveFocus();
  expect(screen.getByRole("radio", { name: "Simple" })).toHaveAttribute("tabindex", "0");
  expect(screen.getByRole("radio", { name: "Off" })).toHaveAttribute("tabindex", "-1");
  await user.tab();
  expect(screen.getByRole("button", { name: "after" })).toHaveFocus();
});

test("a disabled option cannot be chosen", async () => {
  const on = vi.fn();
  render(
    <Segmented
      aria-label="L"
      options={[...opts.slice(0, 2), { value: "heavy", label: "Heavy", disabled: true }]}
      value="off"
      onValueChange={on}
    />,
  );
  await setupUser().click(screen.getByRole("radio", { name: "Heavy" }));
  expect(on).not.toHaveBeenCalled();
});
