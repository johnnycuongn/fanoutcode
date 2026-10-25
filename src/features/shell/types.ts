// The window shell's types.
import type { ProjectLayer as BindingsProjectLayer, ViewId } from "@/lib/bindings";

export type SidebarSection = "tasks" | "docs" | "changes" | "ticket" | "settings";
export type DockTab = "memory" | "team" | "changes" | "overlap" | "ticket" | "test" | "commit";
export type ProjectLayer = BindingsProjectLayer;
/** Tab zero is the Board; the rest are view ids. */
export type TabId = "board" | ViewId;
