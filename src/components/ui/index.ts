// The primitives' public surface. Everything the app composes comes from here; nothing under src/features/ is
// imported by a primitive. PRIMITIVES maps each primitive family to the names it exports (a test pins that every
// listed name exists and every family has a dev example page).
export * from "./cx";
export * from "./tokens";

export * from "./Button";

export * from "./IconButton";

export const PRIMITIVES = {
  IconButton: ["IconButton"],
  Button: ["Button", "buttonVariants"],
} as const satisfies Record<string, readonly string[]>;

export type PrimitiveName = keyof typeof PRIMITIVES;
