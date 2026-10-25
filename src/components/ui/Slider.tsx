import { Slider as SliderPrimitive } from "radix-ui";
import type { ReactElement } from "react";
import { cx, focusRing } from "./cx";

export type SliderProps = {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange(value: number): void;
  /** The accessible name of the thumb. */
  label: string;
  /** Spoken (and shown by assistive tech) in place of the raw number. */
  formatValue?: (value: number) => string;
  disabled?: boolean;
  className?: string;
};

/** One-thumb slider (Radix): arrow keys step by `step`, Home and End jump to the ends. */
export function Slider({
  value,
  min,
  max,
  step = 1,
  onChange,
  label,
  formatValue,
  disabled,
  className,
}: SliderProps): ReactElement {
  return (
    <SliderPrimitive.Root
      value={[value]}
      min={min}
      max={max}
      step={step}
      disabled={disabled}
      onValueChange={(v) => {
        if (v[0] !== undefined) onChange(v[0]);
      }}
      className={cx("relative flex h-5 w-full touch-none items-center select-none data-disabled:opacity-50", className)}
    >
      <SliderPrimitive.Track className="relative h-1 grow overflow-hidden rounded-full bg-well">
        <SliderPrimitive.Range className="absolute h-full rounded-full bg-accent" />
      </SliderPrimitive.Track>
      <SliderPrimitive.Thumb
        aria-label={label}
        aria-valuetext={formatValue?.(value)}
        className={cx("block size-3.5 rounded-full border border-line bg-card shadow-card", focusRing)}
      />
    </SliderPrimitive.Root>
  );
}
