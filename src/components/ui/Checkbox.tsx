import { Check, Minus } from "lucide-react";
import { Checkbox as CheckboxPrimitive } from "radix-ui";
import { useId, type ComponentProps, type ReactElement, type ReactNode } from "react";
import { cx, focusRing } from "./cx";
import { useFieldControl } from "./Field";

export type CheckboxProps = Omit<ComponentProps<typeof CheckboxPrimitive.Root>, "asChild"> & {
  label?: ReactNode;
  description?: ReactNode;
  size?: "sm" | "md";
};

export function Checkbox({ label, description, size = "md", className, id, ...props }: CheckboxProps): ReactElement {
  const generated = useId();
  const field = useFieldControl();
  const controlId = id ?? field?.id ?? generated;
  const descId = description ? `${controlId}-description` : undefined;
  const root = (
    <CheckboxPrimitive.Root
      id={controlId}
      aria-describedby={descId ?? field?.["aria-describedby"]}
      aria-invalid={field?.["aria-invalid"]}
      className={cx(
        "group flex shrink-0 items-center justify-center rounded-card border border-line bg-card text-accent-ink",
        "data-[state=checked]:border-accent data-[state=checked]:bg-accent",
        "data-[state=indeterminate]:border-accent data-[state=indeterminate]:bg-accent",
        "disabled:cursor-not-allowed disabled:opacity-50",
        size === "sm" ? "size-3.5" : "size-4",
        focusRing,
        !label && className,
      )}
      {...props}
    >
      <CheckboxPrimitive.Indicator>
        <Check aria-hidden className="hidden size-3 group-data-[state=checked]:block" />
        <Minus aria-hidden className="hidden size-3 group-data-[state=indeterminate]:block" />
      </CheckboxPrimitive.Indicator>
    </CheckboxPrimitive.Root>
  );
  if (!label) return root;
  return (
    <div className={cx("flex items-start gap-2", className)}>
      {root}
      <div className="flex flex-col">
        <label htmlFor={controlId} className="text-sm leading-4 text-ink">
          {label}
        </label>
        {description && (
          <span id={descId} className="text-xs text-mute">
            {description}
          </span>
        )}
      </div>
    </div>
  );
}
