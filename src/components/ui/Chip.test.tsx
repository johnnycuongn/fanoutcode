import { render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { Chip } from "./Chip";

test("working chip renders a pulsing dot and the ok tone", () => {
  render(<Chip status="working">Working</Chip>);
  expect(screen.getByRole("img", { name: "Working" })).toHaveAttribute("data-pulse", "true");
  expect(screen.getByText("Working").closest("[data-chip]")).toHaveClass("text-ok");
});

test("needs-you chip is amber and idle chip is grey", () => {
  render(
    <>
      <Chip status="needs-you">Plan written, go?</Chip>
      <Chip status="idle">Idle</Chip>
    </>,
  );
  expect(screen.getByText("Plan written, go?").closest("[data-chip]")).toHaveClass("text-warn");
  expect(screen.getByText("Idle").closest("[data-chip]")).toHaveClass("text-mute");
});

test("is a pressable button when selectable", async () => {
  const onClick = vi.fn();
  render(
    <Chip selected={false} onClick={onClick}>
      Needs you
    </Chip>,
  );
  const b = screen.getByRole("button", { name: /Needs you/ });
  expect(b).toHaveAttribute("aria-pressed", "false");
  await setupUser().click(b);
  expect(onClick).toHaveBeenCalledTimes(1);
});

test("selected uses the solid fill", () => {
  render(
    <Chip stage="build" selected onClick={() => {}}>
      Build
    </Chip>,
  );
  expect(screen.getByRole("button")).toHaveClass("bg-stage-build", "text-accent-ink");
});

test("is static when not interactive", () => {
  render(<Chip>Spec</Chip>);
  expect(screen.queryByRole("button")).toBeNull();
});

test("renders the count", () => {
  render(
    <Chip count={3} onClick={() => {}}>
      api
    </Chip>,
  );
  expect(screen.getByText("3")).toBeInTheDocument();
});

test("remove control fires onRemove and not onClick", async () => {
  const onClick = vi.fn();
  const onRemove = vi.fn();
  render(
    <Chip onClick={onClick} onRemove={onRemove}>
      TC-42
    </Chip>,
  );
  await setupUser().click(screen.getByRole("button", { name: "Remove" }));
  expect(onRemove).toHaveBeenCalledTimes(1);
  expect(onClick).not.toHaveBeenCalled();
});

test("a removable chip keeps its main control clickable", async () => {
  const onClick = vi.fn();
  render(
    <Chip onClick={onClick} onRemove={() => {}} removeLabel="Remove TC-42">
      TC-42
    </Chip>,
  );
  await setupUser().click(screen.getByRole("button", { name: "TC-42" }));
  expect(onClick).toHaveBeenCalledTimes(1);
  expect(screen.getByRole("button", { name: "Remove TC-42" })).toBeInTheDocument();
});

test("truncates long labels and exposes the full text as title", () => {
  const long =
    "I could not find the test command; which script should I run for the integration suite in this repository?";
  render(<Chip status="needs-you">{long}</Chip>);
  const label = screen.getByText(long);
  expect(label).toHaveClass("truncate");
  expect(label).toHaveAttribute("title", long);
});
