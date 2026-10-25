import { RadioGroup as RadioGroupPrimitive } from "radix-ui";
import { useId, type ComponentProps, type ReactElement, type ReactNode } from "react";
import { cx, focusRing } from "./cx";

export type RadioOption<T extends string> = {
  value: T;
  label: ReactNode;
  /** A sub-line under the label; it becomes the option's accessible description. */
  description?: ReactNode;
  /** `bad` paints the label in the danger colour (a destructive choice). */
  tone?: "bad";
  disabled?: boolean;
};

export type RadioGroupProps<T extends string> = Omit<ComponentProps<"div">, "onChange" | "defaultValue" | "dir"> & {
  options: readonly RadioOption<T>[];
  value: T;
  onValueChange(value: T): void;
  "aria-label": string;
};

/** A vertical list of exactly-one choices with labels and sub-lines; arrows move and select, Tab leaves the group. */
export function RadioGroup<T extends string>({
  options,
  value,
  onValueChange,
  className,
  ...props
}: RadioGroupProps<T>): ReactElement {
  const id = useId();
  return (
    <RadioGroupPrimitive.Root
      value={value}
      onValueChange={(v) => onValueChange(v as T)}
      orientation="vertical"
      loop
      className={cx("grid gap-1", className)}
      {...props}
    >
      {options.map((o) => {
        const labelId = `${id}-${o.value}-label`;
        const descId = `${id}-${o.value}-desc`;
        return (
          <RadioGroupPrimitive.Item
            key={o.value}
            value={o.value}
            disabled={o.disabled}
            aria-labelledby={labelId}
            aria-describedby={o.description ? descId : undefined}
            className={cx(
              "group flex w-full items-start gap-2 rounded-card border border-line px-2.5 py-2 text-left transition-colors",
              "hover:bg-well data-[state=checked]:border-accent data-[state=checked]:bg-accent/5",
              "disabled:pointer-events-none disabled:opacity-50",
              focusRing,
            )}
          >
            <span
              aria-hidden
              className="mt-0.5 flex size-3.5 shrink-0 items-center justify-center rounded-full border border-line group-data-[state=checked]:border-accent"
            >
              <RadioGroupPrimitive.Indicator className="size-1.5 rounded-full bg-accent" />
            </span>
            <span className="grid min-w-0 gap-0.5">
              <span id={labelId} className={cx("text-sm font-medium", o.tone === "bad" ? "text-bad" : "text-ink")}>
                {o.label}
              </span>
              {o.description && (
                <span id={descId} className="text-xs text-mute">
                  {o.description}
                </span>
              )}
            </span>
          </RadioGroupPrimitive.Item>
        );
      })}
    </RadioGroupPrimitive.Root>
  );
}
