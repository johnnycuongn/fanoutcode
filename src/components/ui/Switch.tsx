import { Switch as SwitchPrimitive } from "radix-ui";
import { useId, type ComponentProps, type ReactElement, type ReactNode } from "react";
import { cx, focusRing } from "./cx";
import { useFieldControl } from "./Field";

export type SwitchProps = Omit<ComponentProps<typeof SwitchPrimitive.Root>, "asChild"> & {
  label?: ReactNode;
  description?: ReactNode;
  size?: "sm" | "md";
  labelPosition?: "start" | "end";
};

export function Switch({
  label,
  description,
  size = "md",
  labelPosition = "end",
  className,
  id,
  ...props
}: SwitchProps): ReactElement {
  const generated = useId();
  const field = useFieldControl();
  const controlId = id ?? field?.id ?? generated;
  const descId = description ? `${controlId}-description` : undefined;
  const root = (
    <SwitchPrimitive.Root
      id={controlId}
      aria-describedby={descId ?? field?.["aria-describedby"]}
      className={cx(
        "inline-flex shrink-0 items-center rounded-full border border-line bg-well transition-colors",
        "data-[state=checked]:border-accent data-[state=checked]:bg-accent",
        "disabled:cursor-not-allowed disabled:opacity-50",
        size === "sm" ? "h-4 w-7" : "h-5 w-9",
        focusRing,
        !label && className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        className={cx(
          "block rounded-full bg-card shadow-card ring-1 ring-line transition-transform",
          size === "sm"
            ? "size-3 translate-x-0.5 data-[state=checked]:translate-x-3.5"
            : "size-4 translate-x-0.5 data-[state=checked]:translate-x-4.5",
        )}
      />
    </SwitchPrimitive.Root>
  );
  if (!label) return root;
  const text = (
    <div className="flex flex-col">
      <label htmlFor={controlId} className="text-sm leading-5 text-ink">
        {label}
      </label>
      {description && (
        <span id={descId} className="text-xs text-mute">
          {description}
        </span>
      )}
    </div>
  );
  return (
    <div className={cx("flex items-start gap-2", className)}>
      {labelPosition === "start" && text}
      {root}
      {labelPosition === "end" && text}
    </div>
  );
}
