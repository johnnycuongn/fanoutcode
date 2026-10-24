// The collapsible sidebar (⌘\): the icon strip on top, then the chosen section.
import type { ReactElement } from "react";
import { Panel } from "@/components/ui";
import type { ProjectId } from "@/lib/bindings";
import { shellStore, type SidebarSection } from "@/features/shell";
import type { ContextDocItem } from "../types";
import { ChangesSection } from "./ChangesSection";
import { DocsSection } from "./DocsSection";
import { IconStrip } from "./IconStrip";
import { SettingsSection } from "./SettingsSection";
import { TasksSection } from "./TasksSection";
import { TicketSection } from "./TicketSection";

export type SidebarProps = { docs?: ContextDocItem[]; onAddPath?: (path: string) => void };

type BodyProps = SidebarProps & { section: SidebarSection; projectId?: ProjectId };

export function SectionBody({ section, projectId, docs, onAddPath }: BodyProps): ReactElement | null {
  switch (section) {
    case "tasks":
      return <TasksSection projectId={projectId} />;
    case "docs":
      return <DocsSection projectId={projectId} docs={docs} onAddPath={onAddPath} />;
    case "changes":
      return <ChangesSection projectId={projectId} />;
    case "ticket":
      return <TicketSection projectId={projectId} />;
    case "settings":
      return <SettingsSection projectId={projectId} />;
  }
}

export function Sidebar(props: SidebarProps): ReactElement | null {
  const open = shellStore.useSelector((s) => s.sidebarOpen);
  const section = shellStore.useSelector((s) => s.sidebarSection);
  const container = shellStore.useSelector((s) => s.container);
  if (!open) return null;
  const projectId = container?.type === "Project" ? container.id : undefined;
  return (
    <Panel
      as="aside"
      variant="plain"
      padding="none"
      aria-label="Sidebar"
      className="flex h-full w-64 shrink-0 flex-col border-r border-line bg-bar"
    >
      <IconStrip />
      <div className="min-h-0 flex-1 overflow-auto py-1">
        <SectionBody section={section} projectId={projectId} {...props} />
      </div>
    </Panel>
  );
}
