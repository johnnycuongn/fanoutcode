import { RadioGroup as RadioGroupPrimitive } from "radix-ui";
import type { ComponentProps, ReactElement, ReactNode } from "react";
import { cx, focusRing } from "./cx";
import { KeyHint } from "./KeyHint";

export type SegmentedOption<T extends string> = {
  value: T;
  label: ReactNode;
  icon?: ReactNode;
  disabled?: boolean;
  /** A chord such as "cmd+1", rendered as a small KeyHint. */
  shortcut?: string;
};

export type SegmentedProps<T extends string> = Omit<ComponentProps<"div">, "onChange" | "defaultValue" | "dir"> & {
  options: readonly SegmentedOption<T>[];
  value: T;
  onValueChange(value: T): void;
  size?: "sm" | "md";
  fullWidth?: boolean;
  "aria-label": string;
};

/** Exactly one option selected; a Radix RadioGroup, so arrows move and select (wrapping) and Tab leaves the group. */
export function Segmented<T extends string>({
  options,
  value,
  onValueChange,
  size = "md",
  fullWidth = false,
  className,
  ...props
}: SegmentedProps<T>): ReactElement {
  return (
    <RadioGroupPrimitive.Root
      value={value}
      onValueChange={(v) => onValueChange(v as T)}
      orientation="horizontal"
      loop
      className={cx(
        "max-w-full flex-wrap gap-0.5 rounded-well border border-line bg-well p-0.5",
        fullWidth ? "flex w-full" : "inline-flex",
        className,
      )}
      {...props}
    >
      {options.map((option) => (
        <RadioGroupPrimitive.Item
          key={option.value}
          value={option.value}
          disabled={option.disabled}
          className={cx(
            "inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-card text-mute transition-colors",
            "hover:text-ink data-[state=checked]:bg-card data-[state=checked]:text-ink data-[state=checked]:shadow-card",
            "disabled:pointer-events-none disabled:opacity-50",
            size === "sm" ? "h-5 px-2 text-xs" : "h-6 px-2.5 text-sm",
            fullWidth && "flex-1",
            focusRing,
          )}
        >
          {option.icon}
          {option.label}
          {option.shortcut && <KeyHint keys={option.shortcut} size="sm" />}
        </RadioGroupPrimitive.Item>
      ))}
    </RadioGroupPrimitive.Root>
  );
}
