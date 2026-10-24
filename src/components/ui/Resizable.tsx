import type { ComponentProps, KeyboardEvent, PointerEvent, ReactElement, ReactNode } from "react";
import { useRef } from "react";
import { cx, focusRing } from "./cx";

export type ResizableProps = Omit<ComponentProps<"div">, "children"> & {
  /** "horizontal" puts `first` and `second` side by side, "vertical" stacks them (LayoutNode::Split). */
  direction: "horizontal" | "vertical";
  /** The share of `first`, clamped to [min, 1 − min]. */
  ratio: number;
  onRatioChange(ratio: number): void;
  min?: number;
  /** Keyboard step; Shift doubles it. */
  step?: number;
  first: ReactNode;
  second: ReactNode;
  handleLabel?: string;
};

const round = (n: number) => Math.round(n * 1000) / 1000;

/** One split with a draggable, keyboard-operable handle (the ARIA separator pattern). */
export function Resizable({
  direction,
  ratio,
  onRatioChange,
  min = 0.1,
  step = 0.05,
  first,
  second,
  handleLabel = "Resize",
  className,
  style,
  ...props
}: ResizableProps): ReactElement {
  const dragging = useRef(false);
  const horizontal = direction === "horizontal";
  const clamp = (n: number) => round(Math.min(Math.max(n, min), 1 - min));
  // A persisted ratio can be out of range (the minimum changed, or a corrupt row): lay out, announce and step from
  // the clamped value, never the stored one.
  const clamped = Number.isFinite(ratio) ? clamp(ratio) : 0.5;
  const emit = (next: number) => {
    if (next !== ratio) onRatioChange(next);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const delta = e.shiftKey ? step * 2 : step;
    const grow = horizontal ? "ArrowRight" : "ArrowDown";
    const shrink = horizontal ? "ArrowLeft" : "ArrowUp";
    let next: number | null = null;
    if (e.key === grow) next = clamp(clamped + delta);
    else if (e.key === shrink) next = clamp(clamped - delta);
    else if (e.key === "Home") next = clamp(min);
    else if (e.key === "End") next = clamp(1 - min);
    else if (e.key === "Enter") next = clamp(0.5);
    if (next === null) return;
    e.preventDefault();
    emit(next);
  };

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    dragging.current = true;
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e: PointerEvent<HTMLDivElement>) => {
    if (!dragging.current) return;
    const rect = e.currentTarget.parentElement?.getBoundingClientRect();
    if (!rect) return;
    const size = horizontal ? rect.width : rect.height;
    if (size <= 0) return;
    const offset = horizontal ? e.clientX - rect.left : e.clientY - rect.top;
    emit(clamp(offset / size));
  };
  const onPointerUp = (e: PointerEvent<HTMLDivElement>) => {
    dragging.current = false;
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
  };

  const template = `minmax(0, ${clamped}fr) 1px minmax(0, ${round(1 - clamped)}fr)`;
  return (
    <div
      className={cx("grid h-full min-h-0 w-full min-w-0", className)}
      style={{ ...(horizontal ? { gridTemplateColumns: template } : { gridTemplateRows: template }), ...style }}
      {...props}
    >
      <div className="min-h-0 min-w-0 overflow-hidden">{first}</div>
      <div
        role="separator"
        tabIndex={0}
        aria-label={handleLabel}
        aria-orientation={horizontal ? "vertical" : "horizontal"}
        aria-valuenow={Math.round(clamped * 100)}
        aria-valuemin={Math.round(min * 100)}
        aria-valuemax={Math.round((1 - min) * 100)}
        onKeyDown={onKeyDown}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        className={cx(
          "relative touch-none bg-line transition-colors hover:bg-accent focus-visible:bg-accent",
          horizontal
            ? "cursor-col-resize before:absolute before:inset-y-0 before:-inset-x-1"
            : "cursor-row-resize before:absolute before:-inset-y-1 before:inset-x-0",
          focusRing,
        )}
      />
      <div className="min-h-0 min-w-0 overflow-hidden">{second}</div>
    </div>
  );
}
