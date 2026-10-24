import type { ReactElement, ReactNode } from "react";
import { Button, type ButtonProps, type ButtonVariant } from "./Button";
import { cx } from "./cx";
import type { Size } from "./tokens";

const square: Record<Size, string> = { sm: "size-6 px-0", md: "size-7 px-0", lg: "size-8 px-0" };

export type IconButtonProps = Omit<ButtonProps, "leading" | "trailing" | "children" | "variant" | "aria-label"> & {
  icon: ReactNode;
  /** The accessible name (aria-label); required because there is no text. */
  label: string;
  variant?: Exclude<ButtonVariant, "link">;
  /** aria-pressed, for toggles such as the sidebar icon strip. */
  active?: boolean;
};

export function IconButton({
  icon,
  label,
  variant = "ghost",
  active,
  size = "md",
  className,
  ...props
}: IconButtonProps): ReactElement {
  return (
    <Button
      variant={variant}
      size={size}
      aria-label={label}
      aria-pressed={active}
      className={cx(square[size], className)}
      {...props}
    >
      {icon}
    </Button>
  );
}
