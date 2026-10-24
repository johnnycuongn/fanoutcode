// The primitives' public surface. Everything the app composes comes from here; nothing under src/features/ is
// imported by a primitive. PRIMITIVES maps each primitive family to the names it exports, in the order of spec
// §15.3 (a test pins the order, that every listed name exists, and that every family has a dev example page).
export * from "./cx";
export * from "./tokens";
export * from "./Button";
export * from "./IconButton";
export * from "./Input";
export * from "./Textarea";
export * from "./Select";
export * from "./Combobox";
export * from "./Checkbox";
export * from "./Switch";
export * from "./Segmented";
export * from "./Tabs";
export * from "./Menu";
export * from "./ContextMenu";
export * from "./Popover";
export * from "./Tooltip";
export * from "./UiProvider";
export * from "./Dialog";
export * from "./Drawer";
export * from "./Toast";
export * from "./Field";
export * from "./Chip";
export * from "./Badge";
export * from "./Avatar";
export * from "./KeyHint";
export * from "./StatusDot";
export * from "./EmptyState";
export * from "./Panel";
export * from "./Resizable";

export const PRIMITIVES = {
  Button: ["Button", "buttonVariants"],
  IconButton: ["IconButton"],
  Input: ["Input"],
  Textarea: ["Textarea"],
  Select: [
    "Select",
    "SelectTrigger",
    "SelectValue",
    "SelectContent",
    "SelectItem",
    "SelectGroup",
    "SelectLabel",
    "SelectSeparator",
  ],
  Combobox: ["Combobox"],
  Checkbox: ["Checkbox"],
  Switch: ["Switch"],
  Segmented: ["Segmented"],
  Tabs: ["Tabs", "TabsList", "TabsTrigger", "TabsContent"],
  Menu: [
    "Menu",
    "MenuTrigger",
    "MenuContent",
    "MenuItem",
    "MenuCheckboxItem",
    "MenuRadioGroup",
    "MenuRadioItem",
    "MenuLabel",
    "MenuSeparator",
    "MenuGroup",
    "MenuSub",
    "MenuSubTrigger",
    "MenuSubContent",
  ],
  ContextMenu: [
    "ContextMenu",
    "ContextMenuTrigger",
    "ContextMenuContent",
    "ContextMenuItem",
    "ContextMenuCheckboxItem",
    "ContextMenuRadioGroup",
    "ContextMenuRadioItem",
    "ContextMenuLabel",
    "ContextMenuSeparator",
    "ContextMenuGroup",
    "ContextMenuSub",
    "ContextMenuSubTrigger",
    "ContextMenuSubContent",
  ],
  Popover: ["Popover", "PopoverTrigger", "PopoverContent", "PopoverAnchor", "PopoverClose"],
  Tooltip: ["Tooltip", "TooltipProvider"],
  Dialog: [
    "Dialog",
    "DialogTrigger",
    "DialogContent",
    "DialogHeader",
    "DialogTitle",
    "DialogDescription",
    "DialogFooter",
    "DialogClose",
  ],
  Drawer: [
    "Drawer",
    "DrawerTrigger",
    "DrawerContent",
    "DrawerHeader",
    "DrawerTitle",
    "DrawerDescription",
    "DrawerFooter",
    "DrawerClose",
  ],
  Toast: ["Toaster", "toast"],
  Field: ["Field", "useFieldControl"],
  Chip: ["Chip"],
  Badge: ["Badge"],
  Avatar: ["Avatar", "AvatarGroup", "initials"],
  KeyHint: ["KeyHint", "formatChord", "chordLabel"],
  StatusDot: ["StatusDot"],
  EmptyState: ["EmptyState"],
  Panel: ["Panel", "PanelHeader", "PanelBody", "PanelFooter"],
  Resizable: ["Resizable"],
} as const satisfies Record<string, readonly string[]>;

export type PrimitiveName = keyof typeof PRIMITIVES;
