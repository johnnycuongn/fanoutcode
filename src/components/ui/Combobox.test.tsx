import { render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { Combobox } from "./Combobox";

const options = [
  { value: "frontend", label: "Frontend" },
  { value: "backend", label: "Backend" },
  { value: "security", label: "Security reviewer" },
];

test("typing filters the options", async () => {
  const user = setupUser();
  render(<Combobox options={options} value={null} onValueChange={() => {}} aria-label="Role" />);
  await user.click(screen.getByRole("combobox", { name: "Role" }));
  await user.type(await screen.findByRole("combobox", { name: "Search…" }), "back");
  expect(screen.getByText("Backend")).toBeInTheDocument();
  expect(screen.queryByText("Frontend")).not.toBeInTheDocument();
});

test("ArrowDown then Enter selects the highlighted option and closes", async () => {
  const on = vi.fn();
  const user = setupUser();
  render(<Combobox options={options} value={null} onValueChange={on} aria-label="Role" />);
  await user.click(screen.getByRole("combobox", { name: "Role" }));
  await screen.findByRole("listbox");
  await user.keyboard("{ArrowDown}{Enter}");
  expect(on).toHaveBeenCalledWith("backend");
  await waitFor(() => expect(screen.queryByRole("listbox")).not.toBeInTheDocument());
});

test("Escape closes without selecting", async () => {
  const on = vi.fn();
  const user = setupUser();
  render(<Combobox options={options} value={null} onValueChange={on} aria-label="Role" />);
  await user.click(screen.getByRole("combobox", { name: "Role" }));
  await screen.findByRole("listbox");
  await user.keyboard("{Escape}");
  await waitFor(() => expect(screen.queryByRole("listbox")).not.toBeInTheDocument());
  expect(on).not.toHaveBeenCalled();
  expect(screen.getByRole("combobox", { name: "Role" })).toHaveFocus();
});

test("shows emptyText when nothing matches", async () => {
  const user = setupUser();
  render(<Combobox options={options} value={null} onValueChange={() => {}} emptyText="No roles" aria-label="Role" />);
  await user.click(screen.getByRole("combobox", { name: "Role" }));
  await user.keyboard("zzz");
  expect(await screen.findByText("No roles")).toBeInTheDocument();
});

test("shows the placeholder when value is not among the options", () => {
  render(
    <Combobox options={options} value="removed" onValueChange={() => {}} placeholder="Pick a role" aria-label="Role" />,
  );
  expect(screen.getByRole("combobox", { name: "Role" })).toHaveTextContent("Pick a role");
});

test("shows the selected label", () => {
  render(<Combobox options={options} value="backend" onValueChange={() => {}} aria-label="Role" />);
  expect(screen.getByRole("combobox", { name: "Role" })).toHaveTextContent("Backend");
});

test("clearable renders a clear control that sets null", async () => {
  const on = vi.fn();
  render(<Combobox options={options} value="backend" onValueChange={on} clearable aria-label="Role" />);
  await setupUser().click(screen.getByRole("button", { name: "Clear" }));
  expect(on).toHaveBeenCalledWith(null);
});
