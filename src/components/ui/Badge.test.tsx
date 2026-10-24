import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { Badge } from "./Badge";

test("defaults to the neutral soft fill", () => {
  render(<Badge>3</Badge>);
  expect(screen.getByText("3")).toHaveClass("bg-well", "text-mute");
});

test("stage badge uses the stage token", () => {
  render(<Badge stage="plan">Plan</Badge>);
  expect(screen.getByText("Plan")).toHaveClass("bg-stage-plan/15", "text-stage-plan");
});

test("solid swatch badge", () => {
  render(
    <Badge swatch="rose" variant="solid">
      api
    </Badge>,
  );
  expect(screen.getByText("api")).toHaveClass("bg-swatch-rose", "text-accent-ink");
});

test("swatch wins over stage, stage wins over tone", () => {
  render(
    <>
      <Badge swatch="rose" stage="plan" tone="bad">
        a
      </Badge>
      <Badge stage="plan" tone="bad">
        b
      </Badge>
    </>,
  );
  expect(screen.getByText("a")).toHaveClass("text-swatch-rose");
  expect(screen.getByText("b")).toHaveClass("text-stage-plan");
});

test("outline keeps the text colour and adds a border", () => {
  render(
    <Badge tone="ok" variant="outline">
      ok
    </Badge>,
  );
  expect(screen.getByText("ok")).toHaveClass("text-ok", "border");
});

test("mono uses the monospace font and keeps the text as title", () => {
  render(<Badge mono>TC-42</Badge>);
  const b = screen.getByText("TC-42");
  expect(b).toHaveClass("font-mono", "truncate");
  expect(b).toHaveAttribute("title", "TC-42");
});

test("with an icon the label still truncates and carries the title", () => {
  const long = "x".repeat(120);
  render(<Badge icon={<svg data-testid="i" />}>{long}</Badge>);
  expect(screen.getByTestId("i")).toBeInTheDocument();
  expect(screen.getByText(long)).toHaveClass("truncate");
  expect(screen.getByText(long)).toHaveAttribute("title", long);
});
