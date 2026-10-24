import { render, screen } from "@testing-library/react";
import { createRef } from "react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { Button } from "./Button";

test('renders a type="button" element with its children', () => {
  render(<Button>Save</Button>);
  expect(screen.getByRole("button", { name: "Save" })).toHaveAttribute("type", "button");
});

test("primary variant uses the accent fill", () => {
  render(<Button variant="primary">Go</Button>);
  expect(screen.getByRole("button")).toHaveClass("bg-accent");
});

test("loading sets aria-busy, disables and swallows clicks", async () => {
  const onClick = vi.fn();
  render(
    <Button loading onClick={onClick}>
      Run
    </Button>,
  );
  const b = screen.getByRole("button");
  expect(b).toHaveAttribute("aria-busy", "true");
  expect(b).toBeDisabled();
  await setupUser().click(b);
  expect(onClick).not.toHaveBeenCalled();
});

test("asChild renders the child element with button classes", () => {
  render(
    <Button asChild>
      <a href="#x">Open</a>
    </Button>,
  );
  expect(screen.getByRole("link", { name: "Open" })).toHaveClass("inline-flex");
});

test("merges className last", () => {
  render(<Button className="mt-2">x</Button>);
  expect(screen.getByRole("button")).toHaveClass("mt-2");
});

test("ref reaches the button element", () => {
  const ref = createRef<HTMLButtonElement>();
  render(<Button ref={ref}>x</Button>);
  expect(ref.current).toBeInstanceOf(HTMLButtonElement);
});
