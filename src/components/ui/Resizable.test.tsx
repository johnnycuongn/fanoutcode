import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { setupUser } from "@/test/ui";
import { Resizable, type ResizableProps } from "./Resizable";

// jsdom has no PointerEvent, so fireEvent would drop clientX and pointerId. Only the drag test needs one.
class TestPointerEvent extends MouseEvent {
  pointerId: number;
  constructor(type: string, init: PointerEventInit = {}) {
    super(type, init);
    this.pointerId = init.pointerId ?? 0;
  }
}
beforeEach(() => {
  vi.stubGlobal("PointerEvent", TestPointerEvent);
});
afterEach(() => {
  vi.unstubAllGlobals();
});

const split = (p: Partial<ResizableProps>) =>
  render(
    <Resizable
      direction="horizontal"
      ratio={0.5}
      onRatioChange={vi.fn()}
      first={<div>A</div>}
      second={<div>B</div>}
      {...p}
    />,
  );

test("renders both panes and a separator carrying the ratio", () => {
  split({ ratio: 0.3 });
  expect(screen.getByText("A")).toBeInTheDocument();
  expect(screen.getByText("B")).toBeInTheDocument();
  expect(screen.getByRole("separator", { name: "Resize" })).toHaveAttribute("aria-valuenow", "30");
});

test("the separator's orientation is perpendicular to the split", () => {
  const { unmount } = split({});
  expect(screen.getByRole("separator")).toHaveAttribute("aria-orientation", "vertical");
  unmount();
  split({ direction: "vertical" });
  expect(screen.getByRole("separator")).toHaveAttribute("aria-orientation", "horizontal");
});

test("ArrowRight grows the first pane by one step", async () => {
  const on = vi.fn();
  split({ onRatioChange: on });
  screen.getByRole("separator").focus();
  await setupUser().keyboard("{ArrowRight}");
  expect(on).toHaveBeenCalledWith(0.55);
});

test("Shift+ArrowRight grows by two steps", async () => {
  const on = vi.fn();
  split({ onRatioChange: on });
  screen.getByRole("separator").focus();
  await setupUser().keyboard("{Shift>}{ArrowRight}{/Shift}");
  expect(on).toHaveBeenCalledWith(0.6);
});

test("ArrowUp shrinks the first pane of a vertical split", async () => {
  const on = vi.fn();
  split({ direction: "vertical", onRatioChange: on });
  screen.getByRole("separator").focus();
  await setupUser().keyboard("{ArrowUp}");
  expect(on).toHaveBeenCalledWith(0.45);
});

test("Home and End clamp to min and 1 − min", async () => {
  const on = vi.fn();
  split({ min: 0.2, onRatioChange: on });
  screen.getByRole("separator").focus();
  const u = setupUser();
  await u.keyboard("{Home}");
  expect(on).toHaveBeenLastCalledWith(0.2);
  await u.keyboard("{End}");
  expect(on).toHaveBeenLastCalledWith(0.8);
});

test("Enter resets to one half", async () => {
  const on = vi.fn();
  split({ ratio: 0.3, onRatioChange: on });
  screen.getByRole("separator").focus();
  await setupUser().keyboard("{Enter}");
  expect(on).toHaveBeenCalledWith(0.5);
});

test("dragging the handle sets the ratio from the pointer position", () => {
  const on = vi.fn();
  const { container } = split({ onRatioChange: on });
  vi.spyOn(container.firstElementChild as HTMLElement, "getBoundingClientRect").mockReturnValue({
    left: 0,
    top: 0,
    width: 400,
    height: 0,
    right: 400,
    bottom: 0,
    x: 0,
    y: 0,
    toJSON() {},
  } as DOMRect);
  const h = screen.getByRole("separator");
  fireEvent.pointerDown(h, { pointerId: 1, clientX: 200 });
  fireEvent.pointerMove(h, { pointerId: 1, clientX: 100 });
  expect(on).toHaveBeenLastCalledWith(0.25);
  fireEvent.pointerMove(h, { pointerId: 1, clientX: 0 });
  expect(on).toHaveBeenLastCalledWith(0.1);
  fireEvent.pointerUp(h, { pointerId: 1 });
  on.mockClear();
  fireEvent.pointerMove(h, { pointerId: 1, clientX: 300 });
  expect(on).not.toHaveBeenCalled();
});

test("clamps an out-of-range ratio for ARIA and steps from the clamped value", async () => {
  const on = vi.fn();
  split({ ratio: 1.4, onRatioChange: on });
  const h = screen.getByRole("separator");
  expect(h).toHaveAttribute("aria-valuenow", "90");
  h.focus();
  await setupUser().keyboard("{ArrowLeft}");
  expect(on).toHaveBeenCalledWith(0.85);
});

test("lays the panes out from the clamped ratio", () => {
  const { container } = split({ ratio: 1.4 });
  expect((container.firstElementChild as HTMLElement).style.gridTemplateColumns).toContain("0.9fr");
});
