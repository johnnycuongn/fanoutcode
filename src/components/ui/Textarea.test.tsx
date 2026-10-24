import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { Field } from "./Field";
import { Textarea } from "./Textarea";

test("autoGrow sets rows from the line count", () => {
  render(<Textarea autoGrow value={"a\nb\nc"} readOnly />);
  expect(screen.getByRole("textbox")).toHaveAttribute("rows", "3");
});

test("autoGrow never exceeds maxRows nor drops below rows", () => {
  const { rerender } = render(<Textarea autoGrow maxRows={2} value={"a\nb\nc"} readOnly />);
  expect(screen.getByRole("textbox")).toHaveAttribute("rows", "2");
  rerender(<Textarea autoGrow rows={4} value="a" readOnly />);
  expect(screen.getByRole("textbox")).toHaveAttribute("rows", "4");
});

test("picks up Field ids", () => {
  render(
    <Field label="Intent">
      <Textarea />
    </Field>,
  );
  expect(screen.getByLabelText("Intent").tagName).toBe("TEXTAREA");
});
