import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { mockMatchMedia } from "@/test/ui";
import { AppMark } from "./AppMark";

const blades = (svg: Element) => svg.querySelector("[data-part='blades']");
const agents = (svg: Element) => svg.querySelector("[data-part='agents']");

test("renders the mark at the given size", () => {
  render(<AppMark size={48} label="Fanout Code" />);
  const svg = screen.getByRole("img", { name: "Fanout Code" });
  expect(svg).toHaveAttribute("width", "48");
  expect(svg).toHaveAttribute("height", "48");
  expect(svg).toHaveAttribute("viewBox", "0 0 1024 1024");
  expect(svg.querySelectorAll("[data-part='agents'] circle")).toHaveLength(5);
});

test("is decorative without a label", () => {
  const { container } = render(<AppMark />);
  const svg = container.querySelector("svg");
  expect(svg).toHaveAttribute("aria-hidden", "true");
  expect(svg).not.toHaveAttribute("role");
});

test("spinning turns the blades and drifts the agents", () => {
  mockMatchMedia(false);
  const { container, rerender } = render(<AppMark />);
  const svg = container.querySelector("svg")!;
  expect(svg).toHaveAttribute("data-spinning", "false");
  expect(blades(svg)).not.toHaveClass("animate-app-mark-spin");
  expect(agents(svg)).not.toHaveClass("animate-app-mark-drift");
  rerender(<AppMark spinning />);
  expect(svg).toHaveAttribute("data-spinning", "true");
  expect(blades(svg)).toHaveClass("animate-app-mark-spin");
  expect(agents(svg)).toHaveClass("animate-app-mark-drift");
});

test("nothing animates under reduced motion", () => {
  mockMatchMedia(true);
  const { container } = render(<AppMark spinning />);
  const svg = container.querySelector("svg")!;
  expect(svg).toHaveAttribute("data-spinning", "false");
  expect(blades(svg)).not.toHaveClass("animate-app-mark-spin");
  expect(agents(svg)).not.toHaveClass("animate-app-mark-drift");
});

test("two marks on one page keep their gradients apart", () => {
  const { container } = render(
    <>
      <AppMark />
      <AppMark />
    </>,
  );
  const ids = [...container.querySelectorAll("linearGradient")].map((g) => g.id);
  expect(new Set(ids).size).toBe(4);
  for (const id of ids) expect(id).toMatch(/^[\w-]+$/);
});
