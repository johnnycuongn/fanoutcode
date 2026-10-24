import { act, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { Toaster, toast } from "./Toast";

test("shows a toast message", async () => {
  render(<Toaster />);
  act(() => {
    toast("Moved to Done");
  });
  expect(await screen.findByText("Moved to Done")).toBeInTheDocument();
});

test("renders an action button that fires its handler", async () => {
  const onClick = vi.fn();
  render(<Toaster />);
  act(() => {
    toast("Archived", { action: { label: "Undo", onClick } });
  });
  await setupUser().click(await screen.findByRole("button", { name: "Undo" }));
  expect(onClick).toHaveBeenCalledTimes(1);
});

test('error toasts carry data-type="error"', async () => {
  render(<Toaster />);
  act(() => {
    toast.error("Hook failed");
  });
  expect((await screen.findByText("Hook failed")).closest("[data-sonner-toast]")).toHaveAttribute("data-type", "error");
});
