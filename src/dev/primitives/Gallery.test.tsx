import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { isDevPrimitivesRoute } from "../../App";
import { Gallery } from "./Gallery";

vi.mock("@/lib/bindings", () => ({ commands: {} }));

test("the index page shows the gallery heading", () => {
  window.location.hash = "";
  render(<Gallery />);
  expect(screen.getByRole("heading", { level: 1, name: "Primitives gallery" })).toBeInTheDocument();
});

test("isDevPrimitivesRoute follows the hash in dev builds", () => {
  window.location.hash = "#/dev/primitives/Button";
  expect(isDevPrimitivesRoute()).toBe(true);
  window.location.hash = "#/other";
  expect(isDevPrimitivesRoute()).toBe(false);
});
