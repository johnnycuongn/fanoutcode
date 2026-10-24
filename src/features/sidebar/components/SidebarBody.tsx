// The chosen section's body. In a workspace window each section is grouped by project root, in workspace order; in
// the Rail layer only the focused project (the first when none is focused) is shown.
import type { ReactElement } from "react";
import type { ProjectId } from "@/lib/bindings";
import { projectsStore, selectProjectList } from "@/lib/stores/projectsStore";
import { selectTaskList, tasksStore } from "@/lib/stores/tasksStore";
import { shellStore, type SidebarSection } from "@/features/shell";
import type { ContextDocItem } from "../types";
import { ChangesSection } from "./ChangesSection";
import { DocsSection } from "./DocsSection";
import { ProjectRoot } from "./ProjectRoot";
import { SettingsSection } from "./SettingsSection";
import { TasksSection } from "./TasksSection";
import { TicketSection } from "./TicketSection";

export type SectionProps = {
  section: SidebarSection;
  projectId?: ProjectId;
  /** Inside a workspace root: one flat task list, the root carrying the count. */
  inRoot?: boolean;
  docs?: ContextDocItem[];
  onAddPath?: (path: string) => void;
};

export function SectionBody({ section, projectId, inRoot, docs, onAddPath }: SectionProps): ReactElement | null {
  switch (section) {
    case "tasks":
      return <TasksSection projectId={projectId} grouped={!inRoot} />;
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

export function SidebarBody(props: Omit<SectionProps, "projectId" | "inRoot">): ReactElement | null {
  const kind = shellStore.useSelector((s) => s.kind);
  const container = shellStore.useSelector((s) => s.container);
  const layer = shellStore.useSelector((s) => s.projectLayer);
  const focused = shellStore.useSelector((s) => s.focusedProject);
  const projects = projectsStore.useSelector(selectProjectList);
  const tasks = tasksStore.useSelector(selectTaskList);
  if (kind !== "workspace") return <SectionBody {...props} projectId={container?.id} />;
  const shown = layer === "Rail" ? projects.filter((p) => p.id === (focused ?? projects[0]?.id)) : projects;
  return (
    <>
      {shown.map((p) => (
        <ProjectRoot
          key={p.id}
          project={p}
          focused={focused === p.id}
          needsYou={tasks.filter((t) => t.project_id === p.id && !t.archived && t.state.type === "NeedsYou").length}
        >
          <SectionBody {...props} projectId={p.id} inRoot />
        </ProjectRoot>
      ))}
    </>
  );
}
