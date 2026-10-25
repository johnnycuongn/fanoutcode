import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { DiffText } from "./DiffText";

test("renders added and removed lines as text with their markers and tones", () => {
  render(<DiffText aria-label="Diff" before={"a\nold\nc"} after={"a\nnew\nc"} />);
  const box = screen.getByRole("region", { name: "Diff" });
  expect(box.querySelector("pre")).toBeTruthy();
  const del = screen.getByText("old").closest("[data-kind]");
  const add = screen.getByText("new").closest("[data-kind]");
  expect(del).toHaveAttribute("data-kind", "del");
  expect(del).toHaveClass("text-bad");
  expect(add).toHaveAttribute("data-kind", "add");
  expect(add).toHaveClass("text-ok");
});

test("markup in the text is shown literally, never parsed", () => {
  render(<DiffText aria-label="Diff" before="" after="<img src=x onerror=alert(1)>" />);
  expect(screen.getByText("<img src=x onerror=alert(1)>")).toBeInTheDocument();
  expect(document.querySelector("img")).toBeNull();
});

test("collapses unchanged runs beyond the context", () => {
  const before = ["x", ...Array.from({ length: 20 }, (_, i) => `k${i}`)].join("\n");
  const after = ["y", ...Array.from({ length: 20 }, (_, i) => `k${i}`)].join("\n");
  render(<DiffText aria-label="Diff" before={before} after={after} context={2} />);
  expect(screen.getByText("k1")).toBeInTheDocument();
  expect(screen.queryByText("k2")).toBeNull();
  expect(screen.getByText("18 unchanged lines")).toBeInTheDocument();
});

test("no change shows the empty text", () => {
  render(<DiffText aria-label="Diff" before="same" after="same" empty="No changes" />);
  expect(screen.getByText("No changes")).toBeInTheDocument();
});
