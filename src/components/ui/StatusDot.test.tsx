import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { mockMatchMedia } from "@/test/ui";
import { StatusDot } from "./StatusDot";

test("working is green and pulses", () => {
  render(<StatusDot status="working" />);
  const d = screen.getByRole("img", { name: "Working" });
  expect(d).toHaveClass("text-ok", "animate-status-pulse");
  expect(d).toHaveAttribute("data-pulse", "true");
});

test("needs-you is amber and still", () => {
  render(<StatusDot status="needs-you" />);
  const d = screen.getByRole("img", { name: "Needs you" });
  expect(d).toHaveClass("text-warn");
  expect(d).not.toHaveClass("animate-status-pulse");
});

test("idle is grey", () => {
  render(<StatusDot status="idle" />);
  expect(screen.getByRole("img", { name: "Idle" })).toHaveClass("text-mute");
});

test("stage colours the dot with the stage token", () => {
  render(<StatusDot stage="review" label="Review" />);
  expect(screen.getByRole("img", { name: "Review" })).toHaveClass("text-stage-review");
});

test("pulse stops under reduced motion", () => {
  mockMatchMedia(true);
  render(<StatusDot status="working" />);
  const d = screen.getByRole("img");
  expect(d).toHaveAttribute("data-pulse", "false");
  expect(d).not.toHaveClass("animate-status-pulse");
});
