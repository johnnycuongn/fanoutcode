import type { ComponentProps, ElementType, ReactElement, ReactNode } from "react";
import { cx } from "./cx";

export type PanelProps = ComponentProps<"section"> & {
  variant?: "card" | "well" | "bar" | "plain";
  padding?: "none" | "sm" | "md";
  as?: "section" | "div" | "aside" | "nav";
};

const VARIANT = {
  card: "rounded-card border border-line bg-card shadow-card",
  well: "rounded-well bg-well",
  bar: "border-b border-line bg-bar",
  plain: "",
} as const;

const PADDING = { none: "p-0", sm: "p-2", md: "p-4" } as const;

/** The surface primitive for the dock, inspector, sidebar sections and cards. */
export function Panel({
  variant = "card",
  padding = "sm",
  as = "section",
  className,
  ...props
}: PanelProps): ReactElement {
  const Tag = as as ElementType;
  return <Tag className={cx("min-w-0 text-ink", VARIANT[variant], PADDING[padding], className)} {...props} />;
}

export type PanelHeaderProps = ComponentProps<"header"> & {
  title?: ReactNode;
  actions?: ReactNode;
  sticky?: boolean;
};

export function PanelHeader({ title, actions, sticky, className, children, ...props }: PanelHeaderProps): ReactElement {
  return (
    <header
      className={cx("flex h-8 items-center justify-between gap-2", sticky && "sticky top-0 z-10 bg-inherit", className)}
      {...props}
    >
      {title && <h3 className="min-w-0 truncate text-sm font-medium text-ink">{title}</h3>}
      {children}
      {actions && <div className="ml-auto flex items-center gap-1">{actions}</div>}
    </header>
  );
}

export function PanelBody({ scroll, className, ...props }: ComponentProps<"div"> & { scroll?: boolean }): ReactElement {
  return <div className={cx(scroll && "min-h-0 overflow-auto", className)} {...props} />;
}

export function PanelFooter({ className, ...props }: ComponentProps<"footer">): ReactElement {
  return <footer className={cx("flex items-center justify-end gap-2 pt-2", className)} {...props} />;
}
