// The Changes section: the files each active task has changed, by task and then by repo. Clicking opens the task
// with its Changes dock tab; a task's list refetches on its `git-changes` event. A file another task also touches
// carries its relation chip, from the task's overlap rows (loaded once here, kept current by `overlap-changed`).
import { useEffect, useState, type ReactElement } from "react";
import { Button, Chip, EmptyState } from "@/components/ui";
import { commands, events, type ProjectId, type RepoChanges, type Task, type TaskId } from "@/lib/bindings";
import { selectTaskList, tasksStore } from "@/lib/stores/tasksStore";
import { fileRelationLabels, overlapStore, refreshOverlaps } from "@/features/overlap";
import { openTaskInCurrentView } from "@/features/shell";

const ACTIVE_STAGES = new Set(["Build", "Review", "Test", "Commit"]);

export function isActiveTask(t: Task): boolean {
  return !t.archived && (t.state.type !== "Idle" || ACTIVE_STAGES.has(t.stage));
}

const repoName = (root: string) => root.split("/").filter(Boolean).pop() ?? root;

function useChanges(ids: TaskId[]): Record<TaskId, RepoChanges[]> {
  const [byTask, setByTask] = useState<Record<TaskId, RepoChanges[]>>({});
  const key = ids.join(",");
  useEffect(() => {
    let live = true;
    const wanted = new Set(key ? key.split(",") : []);
    const fetchOne = async (id: TaskId) => {
      const r = await commands.gitChanges(id);
      if (live && r?.status === "ok") setByTask((s) => ({ ...s, [id]: r.data }));
    };
    for (const id of wanted) void fetchOne(id);
    const off = events.gitChanges.listen((e) => {
      if (wanted.has(e.payload.task_id)) void fetchOne(e.payload.task_id);
    });
    return () => {
      live = false;
      void Promise.resolve(off).then((fn) => fn?.());
    };
  }, [key]);
  return byTask;
}

/** Loads the overlap rows of each shown task not loaded yet; the store refreshes loaded rows on `overlap-changed`. */
function useOverlapRows(ids: TaskId[]) {
  const key = ids.join(",");
  useEffect(() => {
    const loaded = overlapStore.get().byTask;
    for (const id of key ? key.split(",") : []) if (!loaded[id]) void refreshOverlaps(id);
  }, [key]);
  return overlapStore.useSelector((s) => s.byTask);
}

export function ChangesSection({ projectId }: { projectId?: ProjectId }): ReactElement {
  const all = tasksStore.useSelector(selectTaskList);
  const tasks = all.filter((t) => isActiveTask(t) && (!projectId || t.project_id === projectId));
  const byTask = useChanges(tasks.map((t) => t.id));
  const overlaps = useOverlapRows(tasks.map((t) => t.id));
  if (tasks.length === 0) return <EmptyState size="sm" title="No active tasks" description="Changes appear here." />;
  const open = (id: TaskId) => openTaskInCurrentView(id, { dockTab: "changes" });
  return (
    <div className="flex flex-col gap-2 px-1">
      {tasks.map((task) => (
        <section key={task.id} aria-label={task.title}>
          <Button variant="ghost" size="sm" className="w-full justify-start font-medium" onClick={() => open(task.id)}>
            <span className="truncate">{task.title}</span>
          </Button>
          {(byTask[task.id] ?? []).map((repo) => (
            <div key={repo.repo_root} className="pl-2">
              <div className="px-2 text-xs text-mute" title={repo.repo_root}>
                {repoName(repo.repo_root)}
              </div>
              {repo.files.map((f) => (
                <Button
                  key={f.path}
                  variant="ghost"
                  size="sm"
                  className="w-full justify-between gap-2 font-mono text-xs"
                  onClick={() => open(task.id)}
                >
                  <span className="min-w-0 flex-1 truncate text-left">{f.path}</span>
                  {fileRelationLabels(overlaps[task.id] ?? [], repo.repo_root, f.path).map((label) => (
                    <Chip key={label} size="sm" className="shrink-0 font-sans">
                      {label}
                    </Chip>
                  ))}
                  <span className="shrink-0 text-mute">{`+${f.added} −${f.removed}`}</span>
                </Button>
              ))}
            </div>
          ))}
          {byTask[task.id]?.every((r) => r.files.length === 0) && (
            <p className="px-3 text-xs text-mute">No changes yet</p>
          )}
        </section>
      ))}
    </div>
  );
}
