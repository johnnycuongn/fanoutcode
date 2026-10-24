import { Check, ChevronRight, Circle } from "lucide-react";
import { DropdownMenu as DropdownMenuPrimitive } from "radix-ui";
import type { ComponentProps, ReactElement, ReactNode } from "react";
import { useReducedMotion } from "@/lib/motion";
import { cx, popMotion } from "./cx";
import { MenuItemBody, SwatchSquare, menuContentClasses, menuItemClasses, menuItemTone } from "./menuParts";
import type { Swatch } from "./tokens";

export const Menu = DropdownMenuPrimitive.Root;
export const MenuTrigger = DropdownMenuPrimitive.Trigger;
export const MenuGroup = DropdownMenuPrimitive.Group;
export const MenuSub = DropdownMenuPrimitive.Sub;
export const MenuRadioGroup = DropdownMenuPrimitive.RadioGroup;

export type MenuContentProps = ComponentProps<typeof DropdownMenuPrimitive.Content> & { minWidth?: number };

export function MenuContent({
  minWidth = 180,
  className,
  style,
  sideOffset = 4,
  align = "start",
  ...props
}: MenuContentProps): ReactElement {
  const reduced = useReducedMotion();
  return (
    <DropdownMenuPrimitive.Portal>
      <DropdownMenuPrimitive.Content
        sideOffset={sideOffset}
        align={align}
        data-motion={reduced ? "reduced" : "full"}
        style={{ minWidth, ...style }}
        className={cx(menuContentClasses, popMotion(reduced), className)}
        {...props}
      />
    </DropdownMenuPrimitive.Portal>
  );
}

export function MenuSubContent({
  className,
  ...props
}: ComponentProps<typeof DropdownMenuPrimitive.SubContent>): ReactElement {
  const reduced = useReducedMotion();
  return (
    <DropdownMenuPrimitive.Portal>
      <DropdownMenuPrimitive.SubContent
        data-motion={reduced ? "reduced" : "full"}
        className={cx(menuContentClasses, popMotion(reduced), className)}
        {...props}
      />
    </DropdownMenuPrimitive.Portal>
  );
}

export function MenuSubTrigger({
  icon,
  inset,
  className,
  children,
  ...props
}: ComponentProps<typeof DropdownMenuPrimitive.SubTrigger> & { icon?: ReactNode; inset?: boolean }): ReactElement {
  return (
    <DropdownMenuPrimitive.SubTrigger
      className={cx(menuItemClasses, "data-[state=open]:bg-well", inset && "pl-7", className)}
      {...props}
    >
      <MenuItemBody icon={icon}>{children}</MenuItemBody>
      <ChevronRight aria-hidden className="ml-auto size-3.5 text-mute" />
    </DropdownMenuPrimitive.SubTrigger>
  );
}

export type MenuItemProps = ComponentProps<typeof DropdownMenuPrimitive.Item> & {
  icon?: ReactNode;
  /** Right-aligned muted text, e.g. why an item is disabled. */
  hint?: ReactNode;
  /** A chord such as "cmd+e", rendered as a KeyHint. */
  shortcut?: string;
  danger?: boolean;
  inset?: boolean;
};

export function MenuItem({
  icon,
  hint,
  shortcut,
  danger,
  inset,
  className,
  children,
  ...props
}: MenuItemProps): ReactElement {
  return (
    <DropdownMenuPrimitive.Item
      className={cx(menuItemClasses, menuItemTone(danger), inset && "pl-7", className)}
      {...props}
    >
      <MenuItemBody icon={icon} hint={hint} shortcut={shortcut}>
        {children}
      </MenuItemBody>
    </DropdownMenuPrimitive.Item>
  );
}

export function MenuCheckboxItem({
  shortcut,
  className,
  children,
  ...props
}: ComponentProps<typeof DropdownMenuPrimitive.CheckboxItem> & { shortcut?: string }): ReactElement {
  return (
    <DropdownMenuPrimitive.CheckboxItem className={cx(menuItemClasses, "pl-7", className)} {...props}>
      <span className="absolute left-2 flex size-3.5 items-center justify-center">
        <DropdownMenuPrimitive.ItemIndicator>
          <Check aria-hidden className="size-3.5" />
        </DropdownMenuPrimitive.ItemIndicator>
      </span>
      <MenuItemBody shortcut={shortcut}>{children}</MenuItemBody>
    </DropdownMenuPrimitive.CheckboxItem>
  );
}

export function MenuRadioItem({
  swatch,
  className,
  children,
  ...props
}: ComponentProps<typeof DropdownMenuPrimitive.RadioItem> & { swatch?: Swatch }): ReactElement {
  return (
    <DropdownMenuPrimitive.RadioItem className={cx(menuItemClasses, "pl-7", className)} {...props}>
      <span className="absolute left-2 flex size-3.5 items-center justify-center">
        <DropdownMenuPrimitive.ItemIndicator>
          <Circle aria-hidden className="size-1.5 fill-current" />
        </DropdownMenuPrimitive.ItemIndicator>
      </span>
      <MenuItemBody icon={swatch ? <SwatchSquare swatch={swatch} /> : undefined}>{children}</MenuItemBody>
    </DropdownMenuPrimitive.RadioItem>
  );
}

export function MenuLabel({
  inset,
  className,
  ...props
}: ComponentProps<typeof DropdownMenuPrimitive.Label> & { inset?: boolean }): ReactElement {
  return (
    <DropdownMenuPrimitive.Label className={cx("px-2 py-1 text-xs text-mute", inset && "pl-7", className)} {...props} />
  );
}

export function MenuSeparator({
  className,
  ...props
}: ComponentProps<typeof DropdownMenuPrimitive.Separator>): ReactElement {
  return <DropdownMenuPrimitive.Separator className={cx("-mx-1 my-1 h-px bg-line", className)} {...props} />;
}
