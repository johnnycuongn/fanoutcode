import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { Switch } from "./Switch";

test("has role switch and toggles with Space and Enter", async () => {
  const on = vi.fn();
  render(<Switch label="May commit" onCheckedChange={on} />);
  const s = screen.getByRole("switch", { name: "May commit" });
  s.focus();
  const user = setupUser();
  await user.keyboard(" ");
  await user.keyboard("{Enter}");
  expect(on).toHaveBeenCalledTimes(2);
});

test("controlled checked reflects aria-checked", () => {
  render(<Switch checked label="On" />);
  expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "true");
});

test("labelPosition start renders the text before the control", () => {
  render(<Switch label="Sidebar" labelPosition="start" />);
  const label = screen.getByText("Sidebar");
  expect(label.compareDocumentPosition(screen.getByRole("switch")) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
});
