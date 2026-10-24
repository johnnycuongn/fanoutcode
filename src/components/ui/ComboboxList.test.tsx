import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";
import { ComboboxList, type ComboboxListItem } from "./ComboboxList";

const items: ComboboxListItem[] = [
  { value: "t-1", label: "Login flow", group: "Tasks" },
  { value: "t-2", label: "Billing", group: "Tasks", keywords: ["TC-7"] },
  { value: "new", label: "New terminal", group: "New" },
];

describe("ComboboxList", () => {
  test("shows a search field and every item, grouped", () => {
    render(<ComboboxList items={items} onSelect={() => {}} label="Pick a task" />);
    expect(screen.getByRole("combobox", { name: "Pick a task" })).toBeInTheDocument();
    expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual(["Login flow", "Billing", "New terminal"]);
    expect(screen.getByText("Tasks")).toBeInTheDocument();
  });
  test("filters by label and keywords, never by value, and Enter chooses", async () => {
    const onSelect = vi.fn();
    render(<ComboboxList items={items} onSelect={onSelect} label="Pick" />);
    await userEvent.type(screen.getByRole("combobox"), "tc-7");
    expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual(["Billing"]);
    await userEvent.keyboard("{Enter}");
    expect(onSelect).toHaveBeenCalledWith("t-2");
  });
  test("shows the empty text when nothing matches", async () => {
    render(<ComboboxList items={items} onSelect={() => {}} label="Pick" emptyText="Nothing here" />);
    await userEvent.type(screen.getByRole("combobox"), "t-1");
    expect(screen.getByText("Nothing here")).toBeInTheDocument();
  });
  test("clicking an item chooses it", async () => {
    const onSelect = vi.fn();
    render(<ComboboxList items={items} onSelect={onSelect} label="Pick" />);
    await userEvent.click(screen.getByText("New terminal"));
    expect(onSelect).toHaveBeenCalledWith("new");
  });
});
