import { Popover as PopoverPrimitive } from "radix-ui";
import type { ComponentProps, ReactElement } from "react";
import { useReducedMotion } from "@/lib/motion";
import { cx, popMotion } from "./cx";

export const Popover = PopoverPrimitive.Root;
export const PopoverTrigger = PopoverPrimitive.Trigger;
export const PopoverAnchor = PopoverPrimitive.Anchor;
export const PopoverClose = PopoverPrimitive.Close;

export type PopoverContentProps = ComponentProps<typeof PopoverPrimitive.Content> & {
  /** "trigger" sizes the content to the trigger's width. */
  width?: "auto" | "trigger";
  padding?: "none" | "sm" | "md";
};

const PADDING = { none: "p-0", sm: "p-2", md: "p-4" } as const;

export function PopoverContent({
  width = "auto",
  padding = "sm",
  className,
  sideOffset = 6,
  align = "start",
  ...props
}: PopoverContentProps): ReactElement {
  const reduced = useReducedMotion();
  return (
    <PopoverPrimitive.Portal>
      <PopoverPrimitive.Content
        sideOffset={sideOffset}
        align={align}
        data-motion={reduced ? "reduced" : "full"}
        className={cx(
          "z-50 rounded-well border border-line bg-card text-sm text-ink shadow-card outline-none",
          PADDING[padding],
          width === "trigger" && "w-[var(--radix-popover-trigger-width)]",
          popMotion(reduced),
          className,
        )}
        {...props}
      />
    </PopoverPrimitive.Portal>
  );
}
