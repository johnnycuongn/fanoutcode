import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { mockMatchMedia } from "@/test/ui";
import { Button } from "./Button";
import { EmptyState } from "./EmptyState";

vi.mock("./EmptyStateLottie", () => ({
  default: (p: { src: string; reduced: boolean }) => (
    <div data-testid="lottie-stub" data-src={p.src} data-reduced={String(p.reduced)} />
  ),
}));

test("renders title, description and action", () => {
  render(
    <EmptyState
      title="No tasks in Build"
      description="Drag a card here or press Start build."
      action={<Button>New task</Button>}
    />,
  );
  expect(screen.getByRole("heading", { level: 2, name: "No tasks in Build" })).toBeInTheDocument();
  expect(screen.getByText(/Drag a card/)).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "New task" })).toBeInTheDocument();
});

test("renders the icon and no Lottie when there is no animation", () => {
  render(<EmptyState title="Empty" icon={<svg data-testid="icon" />} />);
  expect(screen.getByTestId("icon")).toBeInTheDocument();
  expect(screen.queryByTestId("lottie-stub")).toBeNull();
});

test("loads the Lottie chunk lazily when animation is set", async () => {
  render(<EmptyState title="Empty" animation="/art/empty.lottie" />);
  expect(await screen.findByTestId("lottie-stub")).toHaveAttribute("data-src", "/art/empty.lottie");
});

test("reduced motion renders the animation paused", async () => {
  mockMatchMedia(true);
  render(<EmptyState title="Empty" animation="/art/empty.lottie" />);
  expect(await screen.findByTestId("lottie-stub")).toHaveAttribute("data-reduced", "true");
});

test("art box has a fixed size per size prop", () => {
  const { container } = render(<EmptyState title="x" size="sm" animation="/a.lottie" />);
  expect(container.querySelector("[data-art]")).toHaveClass("size-16");
});

test("the art is decorative", () => {
  const { container } = render(<EmptyState title="x" animation="/a.lottie" />);
  expect(container.querySelector("[data-art]")).toHaveAttribute("aria-hidden", "true");
});
