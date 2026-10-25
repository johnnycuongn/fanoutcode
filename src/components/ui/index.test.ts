import { expect, test } from "vitest";
import * as ui from "./index";
import { PRIMITIVES } from "./index";

test("PRIMITIVES names the twenty-six primitives of spec §15.3 in order, then Slider (Plan 09), AppMark, RadioGroup and DiffText (Plan 11)", () => {
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
    "Slider",
    "AppMark",
    "RadioGroup",
    "DiffText",
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
