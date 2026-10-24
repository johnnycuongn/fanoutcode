import type { ComponentProps, ReactElement, ReactNode } from "react";
import { controlHeight, controlSurface, cx, focusRing } from "./cx";
import { useFieldControl } from "./Field";

export type InputProps = Omit<ComponentProps<"input">, "size"> & {
  size?: "sm" | "md";
  invalid?: boolean;
  leading?: ReactNode;
  trailing?: ReactNode;
  mono?: boolean;
};

export function Input({
  size = "md",
  invalid,
  leading,
  trailing,
  mono,
  className,
  ...props
}: InputProps): ReactElement {
  const field = useFieldControl();
  const isInvalid = invalid || field?.["aria-invalid"] ? true : undefined;
  const input = (adorned: boolean) => (
    <input
      {...field}
      {...props}
      aria-invalid={isInvalid}
      className={cx(
        "min-w-0 text-sm",
        mono && "font-mono",
        adorned
          ? "h-full flex-1 bg-transparent outline-none placeholder:text-mute"
          : cx(controlSurface, controlHeight[size], focusRing, "w-full", className),
      )}
    />
  );
  if (!leading && !trailing) return input(false);
  return (
    <span
      className={cx(
        controlSurface,
        controlHeight[size],
        "flex items-center gap-1.5 focus-within:ring-2 focus-within:ring-accent/60 has-[[aria-invalid=true]]:border-bad",
        "[&>svg]:size-3.5 [&>svg]:shrink-0 [&>svg]:text-mute",
        className,
      )}
    >
      {leading}
      {input(true)}
      {trailing}
    </span>
  );
}
