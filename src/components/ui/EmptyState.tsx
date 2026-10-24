import { lazy, Suspense, type ComponentProps, type ReactElement, type ReactNode } from "react";
import { useReducedMotion } from "@/lib/motion";
import { cx } from "./cx";

// A separate chunk, fetched only when an EmptyState has an animation.
const EmptyStateLottie = lazy(() => import("./EmptyStateLottie"));

export type EmptyStateProps = ComponentProps<"div"> & {
  title: ReactNode;
  description?: ReactNode;
  /** URL of a .lottie file: `import art from "./x.lottie?url"`. */
  animation?: string;
  /** Shown when there is no animation. */
  icon?: ReactNode;
  /** A Button, supplied by the caller. */
  action?: ReactNode;
  /** Art box 64 / 120 / 180 px: column, board, first run. */
  size?: "sm" | "md" | "lg";
};

const ART = {
  sm: { box: "size-16", px: 64 },
  md: { box: "size-30", px: 120 },
  lg: { box: "size-45", px: 180 },
} as const;

export function EmptyState({
  title,
  description,
  animation,
  icon,
  action,
  size = "md",
  className,
  ...props
}: EmptyStateProps): ReactElement {
  const reduced = useReducedMotion();
  const art = ART[size];
  return (
    <div className={cx("flex flex-col items-center gap-2 p-6 text-center text-mute", className)} {...props}>
      {(animation || icon) && (
        // A fixed-size box, so the lazy chunk never shifts the layout.
        <div data-art="" aria-hidden="true" className={cx("flex items-center justify-center", art.box)}>
          {animation ? (
            <Suspense fallback={null}>
              <EmptyStateLottie src={animation} size={art.px} reduced={reduced} />
            </Suspense>
          ) : (
            icon
          )}
        </div>
      )}
      <h2 className="text-sm font-medium text-ink">{title}</h2>
      {description && <p className="max-w-xs text-sm">{description}</p>}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}
