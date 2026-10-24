import type { ComponentProps, ReactElement } from "react";
import { useReducedMotion } from "@/lib/motion";
import { cx } from "./cx";
import { pickFill, statusLabel, statusTone, type AgentStatus, type StageToken, type Swatch, type Tone } from "./tokens";

export type StatusDotProps = ComponentProps<"span"> & {
  /** Colour precedence: status > swatch > stage > tone. */
  status?: AgentStatus;
  stage?: StageToken;
  tone?: Tone;
  swatch?: Swatch;
  size?: "sm" | "md";
  label?: string;
  /** Defaults to `status === "working"`; never true under reduced motion. */
  pulse?: boolean;
};

export function StatusDot({
  status,
  stage,
  tone,
  swatch,
  size = "md",
  label,
  pulse,
  className,
  ...props
}: StatusDotProps): ReactElement {
  const reduced = useReducedMotion();
  const pulsing = (pulse ?? status === "working") && !reduced;
  const fill = status ? pickFill({ tone: statusTone(status) }) : pickFill({ swatch, stage, tone });
  return (
    <span
      role="img"
      aria-label={label ?? (status ? statusLabel(status) : "Status")}
      data-pulse={pulsing ? "true" : "false"}
      className={cx(
        "inline-block shrink-0 rounded-full bg-current",
        size === "sm" ? "size-1.5" : "size-2",
        fill.text,
        pulsing && "animate-status-pulse",
        className,
      )}
      {...props}
    />
  );
}
