import { render, screen } from "@testing-library/react";
import { UiProvider } from "../../components/ui";
import { setupUser } from "@/test/ui";
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

test("the toolbar switches the theme on the document", async () => {
  window.location.hash = "";
  render(
    <UiProvider>
      <Gallery />
    </UiProvider>,
  );
  await setupUser().click(screen.getByRole("radio", { name: "Dark" }));
  expect(document.documentElement.dataset.theme).toBe("dark");
  await setupUser().click(screen.getByRole("radio", { name: "Light" }));
  expect(document.documentElement.dataset.theme).toBe("light");
});

test("the Dense switch shrinks the page text", async () => {
  window.location.hash = "";
  render(
    <UiProvider>
      <Gallery />
    </UiProvider>,
  );
  expect(screen.getByRole("main")).not.toHaveClass("text-xs");
  await setupUser().click(screen.getByRole("switch", { name: "Dense" }));
  expect(screen.getByRole("main")).toHaveClass("text-xs");
});
