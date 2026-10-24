import { cva } from "class-variance-authority";
import { Loader2 } from "lucide-react";
import { Slot } from "radix-ui";
import type { ComponentProps, ReactElement, ReactNode } from "react";
import { controlHeight, controlText, cx, focusRing } from "./cx";
import type { Size } from "./tokens";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger" | "link";

export const buttonVariants = cva(
  cx(
    "inline-flex shrink-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-card font-medium transition-colors",
    "disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50",
    focusRing,
  ),
  {
    variants: {
      variant: {
        primary: "bg-accent text-accent-ink hover:bg-accent/90",
        secondary: "border border-line bg-card text-ink hover:bg-well",
        ghost: "text-ink hover:bg-well",
        danger: "border border-line text-bad hover:bg-bad/10",
        link: "text-accent underline-offset-2 hover:underline",
      },
      size: {
        sm: cx(controlHeight.sm, controlText.sm, "px-2"),
        md: cx(controlHeight.md, controlText.md, "px-2.5"),
        lg: cx(controlHeight.lg, controlText.lg, "px-3"),
      },
    },
    compoundVariants: [{ variant: "link", class: "h-auto px-0" }],
    defaultVariants: { variant: "secondary", size: "md" },
  },
) as (opts?: { variant?: ButtonVariant; size?: Size }) => string;

export type ButtonProps = ComponentProps<"button"> & {
  variant?: ButtonVariant;
  size?: Size;
  /** aria-busy and disabled; a spinner replaces `leading`. */
  loading?: boolean;
  leading?: ReactNode;
  trailing?: ReactNode;
  /** Radix Slot: the child element receives the classes. */
  asChild?: boolean;
};

export function Button({
  variant,
  size,
  loading = false,
  leading,
  trailing,
  asChild = false,
  className,
  children,
  disabled,
  type = "button",
  ...props
}: ButtonProps): ReactElement {
  const classes = cx(buttonVariants({ variant, size }), className);
  if (asChild) {
    return (
      <Slot.Root className={classes} {...props}>
        {children}
      </Slot.Root>
    );
  }
  return (
    <button type={type} className={classes} disabled={disabled || loading} aria-busy={loading || undefined} {...props}>
      {loading ? <Loader2 aria-hidden className="size-3.5 animate-spin motion-reduce:animate-none" /> : leading}
      {children}
      {trailing}
    </button>
  );
}
