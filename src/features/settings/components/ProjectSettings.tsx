// The project settings screen (Task 19 fills it in).
import type { ReactElement } from "react";
import type { ProjectId } from "@/lib/bindings";

export function ProjectSettings({ projectId }: { projectId: ProjectId; compact?: boolean }): ReactElement {
  return <div data-project-settings={projectId} />;
}
