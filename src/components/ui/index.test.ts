import { expect, test } from "vitest";
import * as ui from "./index";
import { PRIMITIVES } from "./index";

test("PRIMITIVES names the twenty-six primitives of spec §15.3 in order", () => {
  expect(Object.keys(PRIMITIVES)).toEqual([
    "Button",
    "IconButton",
    "Input",
    "Textarea",
    "Select",
    "Combobox",
    "Checkbox",
    "Switch",
    "Segmented",
    "Tabs",
    "Menu",
    "ContextMenu",
    "Popover",
    "Tooltip",
    "Dialog",
    "Drawer",
    "Toast",
    "Field",
    "Chip",
    "Badge",
    "Avatar",
    "KeyHint",
    "StatusDot",
    "EmptyState",
    "Panel",
    "Resizable",
  ]);
});

test("the barrel also exports the shared toolkit", () => {
  for (const m of [
    "cx",
    "focusRing",
    "UiProvider",
    "STAGES",
    "statusTone",
    "statusLabel",
    "toneClasses",
    "stageClasses",
    "swatchClasses",
  ]) {
    expect(ui).toHaveProperty(m);
  }
});
