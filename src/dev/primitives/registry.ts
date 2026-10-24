// One example page per primitive, loaded lazily. registry.test.tsx fails until every key of PRIMITIVES has a page.
import { lazy, type ComponentType, type LazyExoticComponent } from "react";
import type { PrimitiveName } from "../../components/ui";

export type ExamplePage = { name: PrimitiveName; blurb: string; Page: LazyExoticComponent<ComponentType> };

function page(name: PrimitiveName, blurb: string, load: () => Promise<{ default: ComponentType }>): ExamplePage {
  return { name, blurb, Page: lazy(load) };
}

export const pages: ExamplePage[] = [
  page("Button", "Primary, secondary, ghost, danger and link buttons", () => import("./pages/ButtonPage")),
  page("IconButton", "Square icon buttons with a required label", () => import("./pages/IconButtonPage")),
  page("KeyHint", "Keyboard chords as glyphs", () => import("./pages/KeyHintPage")),
  page("Badge", "Small status and label badges", () => import("./pages/BadgePage")),
  page("StatusDot", "Pulsing agent-state dot", () => import("./pages/StatusDotPage")),
  page("Field", "Label, control, hint and error wiring", () => import("./pages/FieldPage")),
  page("Input", "Single-line text input with adornments", () => import("./pages/InputPage")),
  page("Textarea", "Multi-line input that can grow", () => import("./pages/TextareaPage")),
  page("Checkbox", "Checkbox with indeterminate state", () => import("./pages/CheckboxPage")),
  page("Switch", "On/off switch", () => import("./pages/SwitchPage")),
  page("Popover", "Anchored floating panel", () => import("./pages/PopoverPage")),
  page("Tooltip", "Hint on hover and focus, with shortcut", () => import("./pages/TooltipPage")),
  page("Select", "Single choice from a short list", () => import("./pages/SelectPage")),
  page("Combobox", "Searchable single choice", () => import("./pages/ComboboxPage")),
  page("Segmented", "Exactly-one-of-N control", () => import("./pages/SegmentedPage")),
  page("Tabs", "Underline, pill and bar tab sets", () => import("./pages/TabsPage")),
  page("Menu", "Dropdown menu with shortcuts and submenus", () => import("./pages/MenuPage")),
  page("ContextMenu", "Right-click menu", () => import("./pages/ContextMenuPage")),
  page("Dialog", "Modal dialog", () => import("./pages/DialogPage")),
  page("Drawer", "Edge-docked sheet (Vaul)", () => import("./pages/DrawerPage")),
  page("Toast", "Transient messages (Sonner)", () => import("./pages/ToastPage")),
];
