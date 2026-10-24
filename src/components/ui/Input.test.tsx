import { render, screen } from "@testing-library/react";
import { createRef } from "react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { Input } from "./Input";

test("forwards value and onChange", async () => {
  const onChange = vi.fn();
  render(<Input value="" onChange={onChange} />);
  await setupUser().type(screen.getByRole("textbox"), "a");
  expect(onChange).toHaveBeenCalledTimes(1);
});

test("renders leading and trailing adornments outside the input", () => {
  render(<Input leading={<span>L</span>} trailing={<span>T</span>} />);
  expect(screen.getByText("L").parentElement).toContainElement(screen.getByRole("textbox"));
  expect(screen.getByText("T")).toBeInTheDocument();
});

test("invalid sets aria-invalid", () => {
  render(<Input invalid />);
  expect(screen.getByRole("textbox")).toHaveAttribute("aria-invalid", "true");
});

test("mono uses the monospace font", () => {
  render(<Input mono />);
  expect(screen.getByRole("textbox")).toHaveClass("font-mono");
});

test("ref reaches the input", () => {
  const ref = createRef<HTMLInputElement>();
  render(<Input ref={ref} />);
  expect(ref.current).toBeInstanceOf(HTMLInputElement);
});
