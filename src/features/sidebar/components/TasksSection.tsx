// The Tasks section: the views, then the tasks grouped Needs you, Running, Idle (newest first in each).
import type { ReactElement } from "react";
import type { ProjectId, Task } from "@/lib/bindings";
import { selectTaskList, tasksByState, tasksStore } from "@/lib/stores/tasksStore";
import { TaskRow } from "./TaskRow";
import { ViewsList } from "./ViewsList";

function Group({ id, title, tasks }: { id: string; title: string; tasks: Task[] }): ReactElement {
  return (
    <section data-testid={`group-${id}`} aria-label={title} className="py-1">
      <h3 className="flex items-center justify-between px-2 py-1 text-xs font-medium text-mute">
        <span>{title}</span>
        <span className="tabular-nums">{tasks.length}</span>
      </h3>
      {tasks.map((t) => (
        <TaskRow key={t.id} task={t} />
      ))}
    </section>
  );
}

/** `grouped: false` (inside a workspace root) lists the same order without group headings. */
export function TasksSection({
  projectId,
  grouped = true,
}: {
  projectId?: ProjectId;
  grouped?: boolean;
}): ReactElement {
  const all = tasksStore.useSelector(selectTaskList);
  const tasks = projectId ? all.filter((t) => t.project_id === projectId) : all;
  const g = tasksByState(tasks);
  if (!grouped)
    return (
      <div className="flex flex-col">
        <ViewsList projectId={projectId} />
        {[...g.needsYou, ...g.running, ...g.idle].map((t) => (
          <TaskRow key={t.id} task={t} />
        ))}
      </div>
    );
  return (
    <div className="flex flex-col">
      <ViewsList projectId={projectId} />
      <Group id="needs-you" title="Needs you" tasks={g.needsYou} />
      <Group id="running" title="Running" tasks={g.running} />
      <Group id="idle" title="Idle" tasks={g.idle} />
    </div>
  );
}
