import { MoreHorizontal } from "lucide-react";
import type { ComponentType, ReactElement, ReactNode } from "react";
import {
  ContextMenu,
  ContextMenuCheckboxItem,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuLabel,
  ContextMenuRadioGroup,
  ContextMenuRadioItem,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
  ContextMenuTrigger,
  IconButton,
  Menu,
  MenuCheckboxItem,
  MenuContent,
  MenuItem,
  MenuLabel,
  MenuRadioGroup,
  MenuRadioItem,
  MenuSub,
  MenuSubContent,
  MenuSubTrigger,
  MenuTrigger,
  type Swatch,
} from "@/components/ui";
import type { MenuItemSpec } from "../menu";

// The same item list renders into the dropdown (`…`) and the context menu; each family supplies its own parts.
type Parts = {
  Item: ComponentType<{
    disabled?: boolean;
    danger?: boolean;
    hint?: ReactNode;
    onSelect?: () => void;
    children?: ReactNode;
  }>;
  Check: ComponentType<{ checked: boolean; disabled?: boolean; onCheckedChange?: () => void; children?: ReactNode }>;
  Label: ComponentType<{ children?: ReactNode }>;
  RadioGroup: ComponentType<{ value: string; children?: ReactNode }>;
  Radio: ComponentType<{ value: string; swatch?: Swatch; onSelect?: () => void; children?: ReactNode }>;
  Sub: ComponentType<{ children?: ReactNode }>;
  SubTrigger: ComponentType<{ children?: ReactNode }>;
  SubContent: ComponentType<{ children?: ReactNode }>;
};

const DROPDOWN: Parts = {
  Item: MenuItem,
  Check: MenuCheckboxItem,
  Label: MenuLabel,
  RadioGroup: MenuRadioGroup,
  Radio: MenuRadioItem,
  Sub: MenuSub,
  SubTrigger: MenuSubTrigger,
  SubContent: MenuSubContent,
};

const CONTEXT: Parts = {
  Item: ContextMenuItem,
  Check: ContextMenuCheckboxItem,
  Label: ContextMenuLabel,
  RadioGroup: ContextMenuRadioGroup,
  Radio: ContextMenuRadioItem,
  Sub: ContextMenuSub,
  SubTrigger: ContextMenuSubTrigger,
  SubContent: ContextMenuSubContent,
};

function renderItem(item: MenuItemSpec, p: Parts): ReactNode {
  const { children } = item;
  if (children && children.length > 0) {
    const checked = children.find((c) => c.checked);
    const choices = children.every((c) => c.checked !== undefined);
    return (
      <p.Sub key={item.id}>
        <p.SubTrigger>{item.label}</p.SubTrigger>
        <p.SubContent>
          {choices ? (
            <p.RadioGroup value={checked?.id ?? ""}>
              {children.map((c) => (
                <p.Radio key={c.id} value={c.id} swatch={c.swatch as Swatch | undefined} onSelect={c.onSelect}>
                  {c.label}
                </p.Radio>
              ))}
            </p.RadioGroup>
          ) : (
            children.map((c) => renderItem(c, p))
          )}
        </p.SubContent>
      </p.Sub>
    );
  }
  if (item.checked !== undefined) {
    return [
      <p.Check key={item.id} checked={item.checked} disabled={item.disabled} onCheckedChange={item.onSelect}>
        {item.label}
      </p.Check>,
      item.hint && <p.Label key={`${item.id}:hint`}>{item.hint}</p.Label>,
    ];
  }
  return (
    <p.Item
      key={item.id}
      disabled={item.disabled}
      danger={item.danger}
      hint={item.hint ?? item.shortcut}
      onSelect={item.onSelect}
    >
      {item.label}
    </p.Item>
  );
}

/** Right-click anywhere on the wrapped card opens the menu. */
export function CardMenu({ items, children }: { items: MenuItemSpec[]; children: ReactNode }): ReactElement {
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div className="contents">{children}</div>
      </ContextMenuTrigger>
      <ContextMenuContent>{items.map((item) => renderItem(item, CONTEXT))}</ContextMenuContent>
    </ContextMenu>
  );
}

/** The `…` button at the right of a card's title row. */
export function CardMenuButton({ items }: { items: MenuItemSpec[] }): ReactElement {
  return (
    <Menu>
      <MenuTrigger asChild>
        <IconButton
          label="Card actions"
          size="sm"
          icon={<MoreHorizontal aria-hidden className="size-3.5" />}
          className="opacity-60 group-hover:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100"
        />
      </MenuTrigger>
      <MenuContent>{items.map((item) => renderItem(item, DROPDOWN))}</MenuContent>
    </Menu>
  );
}
