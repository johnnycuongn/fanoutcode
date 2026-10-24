// The primitives' public surface. Everything the app composes comes from here; nothing under src/features/ is
// imported by a primitive. PRIMITIVES maps each primitive family to the names it exports (a test pins that every
// listed name exists and every family has a dev example page).
export * from "./cx";
export * from "./tokens";

export * from "./Button";

export * from "./IconButton";

export * from "./KeyHint";

export * from "./Badge";

export * from "./StatusDot";

export * from "./Field";

export * from "./Input";

export * from "./Textarea";

export * from "./Checkbox";

export * from "./Switch";

export * from "./Popover";

export * from "./Tooltip";

export * from "./UiProvider";

export * from "./Select";

export const PRIMITIVES = {
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
  Tooltip: ["Tooltip", "TooltipProvider"],
  Popover: ["Popover", "PopoverTrigger", "PopoverContent", "PopoverAnchor", "PopoverClose"],
  Switch: ["Switch"],
  Checkbox: ["Checkbox"],
  Textarea: ["Textarea"],
  Input: ["Input"],
  Field: ["Field", "useFieldControl"],
  StatusDot: ["StatusDot"],
  Badge: ["Badge"],
  KeyHint: ["KeyHint", "formatChord", "chordLabel"],
  IconButton: ["IconButton"],
  Button: ["Button", "buttonVariants"],
} as const satisfies Record<string, readonly string[]>;

export type PrimitiveName = keyof typeof PRIMITIVES;
