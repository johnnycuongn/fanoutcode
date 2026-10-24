import { Tooltip as TooltipPrimitive } from "radix-ui";
import { isValidElement, type ReactElement, type ReactNode } from "react";
import { useReducedMotion } from "@/lib/motion";
import { cx, popMotion } from "./cx";
import { KeyHint } from "./KeyHint";

export type TooltipProps = {
  content: ReactNode;
  /** A chord such as "cmd+shift+a", rendered as a KeyHint. */
  shortcut?: string;
  side?: "top" | "right" | "bottom" | "left";
  align?: "start" | "center" | "end";
  delay?: number;
  /** Renders the children alone. */
  disabled?: boolean;
  /** Must forward a ref (every primitive does). A disabled child is wrapped in a focusable span. */
  children: ReactElement;
};

export function TooltipProvider({
  children,
  delayDuration = 400,
}: {
  children: ReactNode;
  delayDuration?: number;
}): ReactElement {
  return (
    <TooltipPrimitive.Provider delayDuration={delayDuration} skipDelayDuration={200}>
      {children}
    </TooltipPrimitive.Provider>
  );
}

export function Tooltip({
  content,
  shortcut,
  side = "top",
  align = "center",
  delay,
  disabled,
  children,
}: TooltipProps): ReactElement {
  const reduced = useReducedMotion();
  if (disabled) return children;
  // A disabled control fires no pointer or focus events, so the hint that explains why it is disabled would never
  // show: the span receives them instead.
  const childDisabled = isValidElement<{ disabled?: boolean }>(children) && children.props.disabled === true;
  const trigger = childDisabled ? (
    <span tabIndex={0} className="inline-flex rounded-card focus-visible:ring-2 focus-visible:ring-accent/60">
      {children}
    </span>
  ) : (
    children
  );
  return (
    <TooltipPrimitive.Root delayDuration={delay}>
      <TooltipPrimitive.Trigger asChild>{trigger}</TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content
          side={side}
          align={align}
          sideOffset={6}
          data-motion={reduced ? "reduced" : "full"}
          className={cx(
            "z-50 flex max-w-xs items-center gap-2 rounded-card bg-ink px-2 py-1 text-xs text-page shadow-card",
            popMotion(reduced),
          )}
        >
          {content}
          {shortcut && <KeyHint keys={shortcut} size="sm" className="text-page/70" />}
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );
}
