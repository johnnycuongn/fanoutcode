import { Check, ChevronRight, Circle } from "lucide-react";
import { ContextMenu as ContextMenuPrimitive } from "radix-ui";
import type { ComponentProps, ReactElement, ReactNode } from "react";
import { useReducedMotion } from "@/lib/motion";
import { cx, popMotion } from "./cx";
import { MenuItemBody, SwatchSquare, menuContentClasses, menuItemClasses, menuItemTone } from "./menuParts";
import type { Swatch } from "./tokens";

export const ContextMenu = ContextMenuPrimitive.Root;
export const ContextMenuTrigger = ContextMenuPrimitive.Trigger;
export const ContextMenuGroup = ContextMenuPrimitive.Group;
export const ContextMenuSub = ContextMenuPrimitive.Sub;
export const ContextMenuRadioGroup = ContextMenuPrimitive.RadioGroup;

export type ContextMenuContentProps = ComponentProps<typeof ContextMenuPrimitive.Content> & { minWidth?: number };

export function ContextMenuContent({
  minWidth = 180,
  className,
  style,
  sideOffset = 4,
  align = "start",
  ...props
}: ContextMenuContentProps): ReactElement {
  const reduced = useReducedMotion();
  return (
    <ContextMenuPrimitive.Portal>
      <ContextMenuPrimitive.Content
        sideOffset={sideOffset}
        align={align}
        data-motion={reduced ? "reduced" : "full"}
        style={{ minWidth, ...style }}
        className={cx(menuContentClasses, popMotion(reduced), className)}
        {...props}
      />
    </ContextMenuPrimitive.Portal>
  );
}

export function ContextMenuSubContent({
  className,
  ...props
}: ComponentProps<typeof ContextMenuPrimitive.SubContent>): ReactElement {
  const reduced = useReducedMotion();
  return (
    <ContextMenuPrimitive.Portal>
      <ContextMenuPrimitive.SubContent
        data-motion={reduced ? "reduced" : "full"}
        className={cx(menuContentClasses, popMotion(reduced), className)}
        {...props}
      />
    </ContextMenuPrimitive.Portal>
  );
}

export function ContextMenuSubTrigger({
  icon,
  inset,
  className,
  children,
  ...props
}: ComponentProps<typeof ContextMenuPrimitive.SubTrigger> & { icon?: ReactNode; inset?: boolean }): ReactElement {
  return (
    <ContextMenuPrimitive.SubTrigger
      className={cx(menuItemClasses, "data-[state=open]:bg-well", inset && "pl-7", className)}
      {...props}
    >
      <MenuItemBody icon={icon}>{children}</MenuItemBody>
      <ChevronRight aria-hidden className="ml-auto size-3.5 text-mute" />
    </ContextMenuPrimitive.SubTrigger>
  );
}

export type ContextMenuItemProps = ComponentProps<typeof ContextMenuPrimitive.Item> & {
  icon?: ReactNode;
  /** Right-aligned muted text, e.g. why an item is disabled. */
  hint?: ReactNode;
  /** A chord such as "cmd+e", rendered as a KeyHint. */
  shortcut?: string;
  danger?: boolean;
  inset?: boolean;
};

export function ContextMenuItem({
  icon,
  hint,
  shortcut,
  danger,
  inset,
  className,
  children,
  ...props
}: ContextMenuItemProps): ReactElement {
  return (
    <ContextMenuPrimitive.Item
      className={cx(menuItemClasses, menuItemTone(danger), inset && "pl-7", className)}
      {...props}
    >
      <MenuItemBody icon={icon} hint={hint} shortcut={shortcut}>
        {children}
      </MenuItemBody>
    </ContextMenuPrimitive.Item>
  );
}

export function ContextMenuCheckboxItem({
  shortcut,
  className,
  children,
  ...props
}: ComponentProps<typeof ContextMenuPrimitive.CheckboxItem> & { shortcut?: string }): ReactElement {
  return (
    <ContextMenuPrimitive.CheckboxItem className={cx(menuItemClasses, "pl-7", className)} {...props}>
      <span className="absolute left-2 flex size-3.5 items-center justify-center">
        <ContextMenuPrimitive.ItemIndicator>
          <Check aria-hidden className="size-3.5" />
        </ContextMenuPrimitive.ItemIndicator>
      </span>
      <MenuItemBody shortcut={shortcut}>{children}</MenuItemBody>
    </ContextMenuPrimitive.CheckboxItem>
  );
}

export function ContextMenuRadioItem({
  swatch,
  className,
  children,
  ...props
}: ComponentProps<typeof ContextMenuPrimitive.RadioItem> & { swatch?: Swatch }): ReactElement {
  return (
    <ContextMenuPrimitive.RadioItem className={cx(menuItemClasses, "pl-7", className)} {...props}>
      <span className="absolute left-2 flex size-3.5 items-center justify-center">
        <ContextMenuPrimitive.ItemIndicator>
          <Circle aria-hidden className="size-1.5 fill-current" />
        </ContextMenuPrimitive.ItemIndicator>
      </span>
      <MenuItemBody icon={swatch ? <SwatchSquare swatch={swatch} /> : undefined}>{children}</MenuItemBody>
    </ContextMenuPrimitive.RadioItem>
  );
}

export function ContextMenuLabel({
  inset,
  className,
  ...props
}: ComponentProps<typeof ContextMenuPrimitive.Label> & { inset?: boolean }): ReactElement {
  return (
    <ContextMenuPrimitive.Label className={cx("px-2 py-1 text-xs text-mute", inset && "pl-7", className)} {...props} />
  );
}

export function ContextMenuSeparator({
  className,
  ...props
}: ComponentProps<typeof ContextMenuPrimitive.Separator>): ReactElement {
  return <ContextMenuPrimitive.Separator className={cx("-mx-1 my-1 h-px bg-line", className)} {...props} />;
}
