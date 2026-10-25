// ⌘P: what the picked pane shows. Every live task of the container (in a workspace with its home project's badge),
// then a new terminal (one per project in a workspace). Filters by title and Jira key.
import type { ReactElement } from "react";
import {
  Badge,
  Chip,
  ComboboxList,
  Dialog,
  DialogContent,
  DialogTitle,
  toast,
  type ComboboxListItem,
} from "@/components/ui";
import { commands, type Project, type Task } from "@/lib/bindings";
import { agentStatus, asSwatch, stageToken } from "@/lib/format";
import { projectsStore, selectProjectList } from "@/lib/stores/projectsStore";
import { selectTaskList, tasksStore } from "@/lib/stores/tasksStore";
import { shellStore } from "@/features/shell";
import { taskContent, terminalContent } from "../layout";
import { closePicker, setPaneContent, viewsStore } from "../store";

type Entry = ComboboxListItem & { task?: Task; project?: Project };

function TaskRow({ task, project, workspace }: { task: Task; project?: Project; workspace: boolean }) {
  return (
    <>
      <span className="min-w-0 truncate">{task.title}</span>
      {task.jira_key && (
        <Badge size="sm" mono>
          {task.jira_key}
        </Badge>
      )}
      <span className="ml-auto flex shrink-0 items-center gap-1">
        {workspace && project && (
          <Badge size="sm" swatch={asSwatch(project.color)}>
            {project.name}
          </Badge>
        )}
        <Chip size="sm" stage={stageToken(task.stage)}>
          {task.stage}
        </Chip>
        <Chip size="sm" status={agentStatus(task.state)}>
          {task.state.type === "NeedsYou" ? "needs you" : task.state.type === "Working" ? "working" : "idle"}
        </Chip>
      </span>
    </>
  );
}

export function TaskPicker(): ReactElement | null {
  const pickerFor = viewsStore.useSelector((s) => s.pickerFor);
  const workspace = shellStore.useSelector((s) => s.kind === "workspace");
  const tasks = tasksStore.useSelector(selectTaskList);
  const projects = projectsStore.useSelector(selectProjectList);
  if (!pickerFor) return null;
  const byId = Object.fromEntries(projects.map((p) => [p.id, p]));

  const entries: Entry[] = [
    ...tasks
      .filter((t) => !t.archived)
      .map((t) => ({
        value: `task:${t.id}`,
        label: t.title,
        keywords: t.jira_key ? [t.jira_key] : [],
        group: "Tasks",
        task: t,
        project: byId[t.project_id],
      })),
    ...(workspace ? projects : projects.slice(0, 1)).map((p) => ({
      value: `terminal:${p.id}`,
      label: workspace ? `New terminal in ${p.name}` : "New terminal",
      group: "New",
      project: p,
    })),
  ];

  const choose = async (value: string) => {
    const entry = entries.find((e) => e.value === value);
    if (!entry) return;
    const { viewId, pane } = pickerFor;
    if (entry.task) {
      setPaneContent(viewId, pane, taskContent(entry.task.id));
      return;
    }
    if (!entry.project) return;
    closePicker();
    const r = await commands.sessionsTerminal(entry.project.id);
    if (r.status === "ok") setPaneContent(viewId, pane, terminalContent(entry.project.id, r.data));
    else toast.error(`Couldn't open a terminal: ${r.error}`);
  };

  return (
    <Dialog open onOpenChange={(open) => !open && closePicker()}>
      <DialogContent size="lg" hideClose className="p-0">
        <DialogTitle className="sr-only">Pick a task</DialogTitle>
        <ComboboxList
          label="Pick a task or terminal"
          placeholder="Pick a task or open a terminal…"
          emptyText="No task matches"
          items={entries}
          onSelect={(v) => void choose(v)}
          renderItem={(item) => {
            const e = item as Entry;
            return e.task ? (
              <TaskRow task={e.task} project={e.project} workspace={workspace} />
            ) : (
              <span>{e.label}</span>
            );
          }}
        />
      </DialogContent>
    </Dialog>
  );
}
