import { Check, ChevronDown, ChevronUp } from "lucide-react";
import { Select as SelectPrimitive } from "radix-ui";
import type { ComponentProps, ReactElement, ReactNode } from "react";
import { useReducedMotion } from "@/lib/motion";
import { controlHeight, controlSurface, cx, focusRing, popMotion } from "./cx";
import { useFieldControl } from "./Field";

export const Select = SelectPrimitive.Root;
export const SelectValue = SelectPrimitive.Value;
export const SelectGroup = SelectPrimitive.Group;

export type SelectTriggerProps = ComponentProps<typeof SelectPrimitive.Trigger> & {
  size?: "sm" | "md";
  invalid?: boolean;
};

export function SelectTrigger({
  size = "md",
  invalid,
  className,
  children,
  ...props
}: SelectTriggerProps): ReactElement {
  const field = useFieldControl();
  return (
    <SelectPrimitive.Trigger
      {...field}
      aria-invalid={invalid || field?.["aria-invalid"] ? true : undefined}
      className={cx(
        controlSurface,
        controlHeight[size],
        "inline-flex w-full items-center justify-between gap-2 text-left text-sm data-[placeholder]:text-mute",
        focusRing,
        className,
      )}
      {...props}
    >
      <span className="min-w-0 truncate">{children}</span>
      <SelectPrimitive.Icon asChild>
        <ChevronDown aria-hidden className="size-3.5 shrink-0 text-mute" />
      </SelectPrimitive.Icon>
    </SelectPrimitive.Trigger>
  );
}

export type SelectContentProps = ComponentProps<typeof SelectPrimitive.Content>;

export function SelectContent({
  className,
  children,
  position = "popper",
  sideOffset = 4,
  ...props
}: SelectContentProps): ReactElement {
  const reduced = useReducedMotion();
  return (
    <SelectPrimitive.Portal>
      <SelectPrimitive.Content
        position={position}
        sideOffset={sideOffset}
        data-motion={reduced ? "reduced" : "full"}
        className={cx(
          "z-50 max-h-72 overflow-hidden rounded-well border border-line bg-card text-sm text-ink shadow-card",
          popMotion(reduced),
          className,
        )}
        {...props}
      >
        <SelectPrimitive.ScrollUpButton className="flex h-6 items-center justify-center text-mute">
          <ChevronUp aria-hidden className="size-3.5" />
        </SelectPrimitive.ScrollUpButton>
        <SelectPrimitive.Viewport
          className={cx("p-1", position === "popper" && "min-w-[var(--radix-select-trigger-width)]")}
        >
          {children}
        </SelectPrimitive.Viewport>
        <SelectPrimitive.ScrollDownButton className="flex h-6 items-center justify-center text-mute">
          <ChevronDown aria-hidden className="size-3.5" />
        </SelectPrimitive.ScrollDownButton>
      </SelectPrimitive.Content>
    </SelectPrimitive.Portal>
  );
}

export type SelectItemProps = ComponentProps<typeof SelectPrimitive.Item> & { hint?: ReactNode };

export function SelectItem({ hint, className, children, ...props }: SelectItemProps): ReactElement {
  return (
    <SelectPrimitive.Item
      className={cx(
        "relative flex h-7 cursor-default select-none items-center gap-2 rounded-card pr-2 pl-7 outline-none",
        "data-[highlighted]:bg-well data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
        className,
      )}
      {...props}
    >
      <span className="absolute left-2 flex size-3.5 items-center justify-center">
        <SelectPrimitive.ItemIndicator>
          <Check aria-hidden className="size-3.5" />
        </SelectPrimitive.ItemIndicator>
      </span>
      <SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
      {hint && <span className="ml-auto text-xs text-mute">{hint}</span>}
    </SelectPrimitive.Item>
  );
}

export function SelectLabel({ className, ...props }: ComponentProps<typeof SelectPrimitive.Label>): ReactElement {
  return <SelectPrimitive.Label className={cx("px-2 py-1 text-xs text-mute", className)} {...props} />;
}

export function SelectSeparator({
  className,
  ...props
}: ComponentProps<typeof SelectPrimitive.Separator>): ReactElement {
  return <SelectPrimitive.Separator className={cx("-mx-1 my-1 h-px bg-line", className)} {...props} />;
}
