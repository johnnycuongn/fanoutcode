import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { IconButton } from "./IconButton";

test("exposes label as the accessible name", () => {
  render(<IconButton icon={<svg />} label="Archive" />);
  expect(screen.getByRole("button", { name: "Archive" })).toBeInTheDocument();
});

test("active sets aria-pressed", () => {
  render(<IconButton icon={<svg />} label="Tasks" active />);
  expect(screen.getByRole("button")).toHaveAttribute("aria-pressed", "true");
});

test("aria-pressed is absent when active is not given", () => {
  render(<IconButton icon={<svg />} label="Tasks" />);
  expect(screen.getByRole("button")).not.toHaveAttribute("aria-pressed");
});

test("is square at md", () => {
  render(<IconButton icon={<svg />} label="x" />);
  expect(screen.getByRole("button")).toHaveClass("size-7");
});
