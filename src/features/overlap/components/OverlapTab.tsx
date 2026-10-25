// The Overlap dock tab (§7). A placeholder until the file list and decision pane land (Plan 11 Task 16).
import type { ReactElement } from "react";
import { EmptyState } from "@/components/ui";
import type { Project, Task } from "@/lib/bindings";

export interface OverlapTabProps {
  task: Task;
  projects: Project[];
}

export function OverlapTab({ task }: OverlapTabProps): ReactElement {
  return <EmptyState size="sm" title="Overlaps load here" data-task={task.id} />;
}
