import type { ComponentProps, ReactElement } from "react";
import { controlSurface, cx, focusRing } from "./cx";
import { useFieldControl } from "./Field";

export type TextareaProps = ComponentProps<"textarea"> & {
  size?: "sm" | "md";
  invalid?: boolean;
  mono?: boolean;
  /** rows = clamp(lineCount(value), rows ?? 2, maxRows ?? 8) */
  autoGrow?: boolean;
  maxRows?: number;
};

function lineCount(value: ComponentProps<"textarea">["value"]): number {
  return String(Array.isArray(value) ? value.join("\n") : (value ?? "")).split("\n").length;
}

export function Textarea({
  size = "md",
  invalid,
  mono,
  autoGrow,
  maxRows,
  rows,
  className,
  ...props
}: TextareaProps): ReactElement {
  const field = useFieldControl();
  const grown = autoGrow
    ? Math.min(Math.max(lineCount(props.value ?? props.defaultValue), rows ?? 2), maxRows ?? 8)
    : rows;
  return (
    <textarea
      {...field}
      {...props}
      rows={grown}
      aria-invalid={invalid || field?.["aria-invalid"] ? true : undefined}
      className={cx(
        controlSurface,
        "w-full py-1.5",
        size === "sm" ? "text-xs" : "text-sm",
        mono && "font-mono",
        autoGrow && "resize-none",
        focusRing,
        className,
      )}
    />
  );
}
