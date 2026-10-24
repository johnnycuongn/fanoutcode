import type { ComponentProps, ReactElement, ReactNode } from "react";
import { cx } from "./cx";
import { pickFill, type StageToken, type Swatch, type Tone } from "./tokens";

export type BadgeProps = ComponentProps<"span"> & {
  /** Colour precedence: swatch > stage > tone; default tone is neutral. */
  tone?: Tone;
  stage?: StageToken;
  swatch?: Swatch;
  variant?: "soft" | "solid" | "outline";
  size?: "sm" | "md";
  mono?: boolean;
  icon?: ReactNode;
  dot?: boolean;
};

/** A small label. Long text truncates with an ellipsis and the full text stays available in `title`. */
export function Badge({
  tone,
  stage,
  swatch,
  variant = "soft",
  size = "md",
  mono = false,
  icon,
  dot = false,
  className,
  children,
  title,
  ...props
}: BadgeProps): ReactElement {
  const fill = pickFill({ swatch, stage, tone });
  const text = typeof children === "string" || typeof children === "number" ? String(children) : undefined;
  const classes = cx(
    "max-w-full truncate rounded-card align-middle font-medium",
    size === "sm" ? "px-1 text-[10px] leading-4" : "px-1.5 text-xs leading-5",
    mono && "font-mono",
    variant === "outline" ? cx(fill.text, "border border-current/30") : fill[variant],
    className,
  );
  if (!icon && !dot) {
    return (
      <span className={cx("inline-block", classes)} title={title ?? text} {...props}>
        {children}
      </span>
    );
  }
  return (
    <span className={cx("inline-flex items-center gap-1", classes)} {...props}>
      {dot && <span aria-hidden className="size-1.5 shrink-0 rounded-full bg-current" />}
      {icon}
      <span className="truncate" title={title ?? text}>
        {children}
      </span>
    </span>
  );
}
