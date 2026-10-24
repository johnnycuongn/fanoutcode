import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { Field } from "./Field";
import { Input } from "./Input";

test("links the label to the control inside it", () => {
  render(
    <Field label="Title">
      <Input />
    </Field>,
  );
  expect(screen.getByLabelText("Title")).toBeInstanceOf(HTMLInputElement);
});

test("describes the control with hint and error", () => {
  render(
    <Field label="Title" hint="Keep it short" error="Required">
      <Input />
    </Field>,
  );
  expect(screen.getByRole("textbox")).toHaveAccessibleDescription("Keep it short Required");
});

test("marks the control invalid and announces the error", () => {
  render(
    <Field label="Title" error="Required">
      <Input />
    </Field>,
  );
  expect(screen.getByRole("textbox")).toHaveAttribute("aria-invalid", "true");
  expect(screen.getByRole("alert")).toHaveTextContent("Required");
});

test("required adds aria-required and a visual marker", () => {
  render(
    <Field label="Title" required>
      <Input />
    </Field>,
  );
  expect(screen.getByRole("textbox")).toHaveAttribute("aria-required", "true");
  expect(screen.getByText("*")).toHaveAttribute("aria-hidden", "true");
});

test("row layout places label and control side by side", () => {
  const { container } = render(
    <Field label="Theme" layout="row">
      <Input />
    </Field>,
  );
  expect(container.firstChild).toHaveClass("grid-cols-[minmax(8rem,1fr)_2fr]");
});

test("a control outside a Field still renders with its own props", () => {
  render(<Input id="solo" />);
  expect(screen.getByRole("textbox")).toHaveAttribute("id", "solo");
});
