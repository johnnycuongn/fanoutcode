// The Settings section: the project's settings screen, inside the sidebar.
import { lazy, Suspense, type ReactElement } from "react";
import type { ProjectId } from "@/lib/bindings";

const ProjectSettings = lazy(() => import("@/features/settings").then((m) => ({ default: m.ProjectSettings })));

export function SettingsSection({ projectId }: { projectId?: ProjectId }): ReactElement | null {
  if (!projectId) return null;
  return (
    <Suspense fallback={null}>
      <ProjectSettings projectId={projectId} compact />
    </Suspense>
  );
}
