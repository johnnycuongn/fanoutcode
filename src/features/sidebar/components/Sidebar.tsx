// The collapsible sidebar (⌘\): the icon strip on top, then the chosen section.
import type { ReactElement } from "react";
import { Panel } from "@/components/ui";
import { shellStore } from "@/features/shell";
import type { ContextDocItem } from "../types";
import { IconStrip } from "./IconStrip";
import { SidebarBody } from "./SidebarBody";

export type SidebarProps = { docs?: ContextDocItem[]; onAddPath?: (path: string) => void };

export function Sidebar(props: SidebarProps): ReactElement | null {
  const open = shellStore.useSelector((s) => s.sidebarOpen);
  const section = shellStore.useSelector((s) => s.sidebarSection);
  if (!open) return null;
  return (
    <Panel
      as="aside"
      variant="plain"
      padding="none"
      aria-label="Sidebar"
      className="flex h-full w-64 shrink-0 flex-col border-r border-line bg-bar"
    >
      <IconStrip />
      <div className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden py-1">
        <SidebarBody section={section} {...props} />
      </div>
    </Panel>
  );
}
