import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { Avatar, AvatarGroup, initials } from "./Avatar";

test.each([
  ["frontend", "FR"],
  ["best-practice reviewer", "BR"],
  ["Johnny Nguyen", "JN"],
  ["codebase-consistency reviewer", "CR"],
  ["code-review", "CR"],
])("initials(%s) → %s", (n, i) => expect(initials(n)).toBe(i));

test("renders initials when there is no image", () => {
  render(<Avatar name="security reviewer" />);
  expect(screen.getByRole("img", { name: "security reviewer" })).toHaveTextContent("SR");
});

test("status renders a dot overlay", () => {
  render(<Avatar name="frontend" status="working" />);
  expect(screen.getByRole("img", { name: "Working" })).toBeInTheDocument();
});

test("AvatarGroup shows +N past max", () => {
  render(
    <AvatarGroup max={2}>
      <Avatar name="a" />
      <Avatar name="b" />
      <Avatar name="c" />
      <Avatar name="d" />
    </AvatarGroup>,
  );
  expect(screen.getByText("+2")).toBeInTheDocument();
  expect(screen.getAllByRole("img")).toHaveLength(2);
});

test("AvatarGroup shows everything within max", () => {
  render(
    <AvatarGroup>
      <Avatar name="a" />
      <Avatar name="b" />
    </AvatarGroup>,
  );
  expect(screen.getAllByRole("img")).toHaveLength(2);
  expect(screen.queryByText(/^\+/)).toBeNull();
});
