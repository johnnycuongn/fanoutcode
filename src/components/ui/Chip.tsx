import { X } from "lucide-react";
import type { ComponentProps, MouseEventHandler, ReactElement, ReactNode } from "react";
import { cx, focusRing } from "./cx";
import { StatusDot } from "./StatusDot";
import { pickFill, statusTone, type AgentStatus, type StageToken, type Swatch, type Tone } from "./tokens";

export type ChipProps = Omit<ComponentProps<"button">, "onClick"> & {
  /** Colour precedence: status (tone from the status, plus a StatusDot) > swatch > stage > tone. */
  tone?: Tone;
  stage?: StageToken;
  swatch?: Swatch;
  status?: AgentStatus;
  size?: "sm" | "md";
  /** Filter chips: aria-pressed; selected renders the solid fill, otherwise the soft one. */
  selected?: boolean;
  count?: number;
  /** Leading icon when there is no status dot. */
  icon?: ReactNode;
  onClick?: MouseEventHandler<HTMLButtonElement>;
  onRemove?(): void;
  removeLabel?: string;
};

/** Status, filter, stepper and attention-strip chips. Long labels truncate; the full text stays in `title`. */
export function Chip({
  tone,
  stage,
  swatch,
  status,
  size = "md",
  selected,
  count,
  icon,
  onClick,
  onRemove,
  removeLabel = "Remove",
  className,
  children,
  type = "button",
  ...props
}: ChipProps): ReactElement {
  const fill = status ? pickFill({ tone: statusTone(status) }) : pickFill({ swatch, stage, tone });
  const text = typeof children === "string" || typeof children === "number" ? String(children) : undefined;
  const interactive = Boolean(onClick) || selected !== undefined;
  const rootClasses = cx(
    "inline-flex h-5 max-w-full items-center gap-1 rounded-full border border-transparent px-2 text-xs",
    size === "md" && "h-6 text-sm",
    selected ? fill.solid : fill.soft,
    className,
  );
  const body = (
    <>
      {status ? <StatusDot status={status} size="sm" /> : icon}
      <span className="truncate" title={text}>
        {children}
      </span>
      {count !== undefined && <span className="tabular-nums opacity-80">{count}</span>}
    </>
  );

  if (!onRemove) {
    return interactive ? (
      <button
        type={type}
        data-chip=""
        aria-pressed={selected}
        onClick={onClick}
        className={cx(rootClasses, "hover:opacity-90", focusRing)}
        {...props}
      >
        {body}
      </button>
    ) : (
      <span data-chip="" className={rootClasses} {...(props as ComponentProps<"span">)}>
        {body}
      </span>
    );
  }

  // A button inside a button is invalid, so a removable chip is a group of two siblings.
  return (
    <span role="group" data-chip="" className={cx(rootClasses, "pr-1")}>
      {interactive ? (
        <button
          type={type}
          aria-pressed={selected}
          onClick={onClick}
          className={cx("inline-flex min-w-0 items-center gap-1 rounded-full", focusRing)}
          {...props}
        >
          {body}
        </button>
      ) : (
        <span className="inline-flex min-w-0 items-center gap-1" {...(props as ComponentProps<"span">)}>
          {body}
        </span>
      )}
      <button
        type="button"
        aria-label={removeLabel}
        onClick={onRemove}
        className={cx("flex size-4 shrink-0 items-center justify-center rounded-full hover:bg-ink/10", focusRing)}
      >
        <X aria-hidden className="size-3" />
      </button>
    </span>
  );
}
