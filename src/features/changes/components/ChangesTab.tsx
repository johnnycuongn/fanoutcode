// The Changes tab (§6.4): the file list on the left, the diff editor on the right, the unified or side-by-side switch,
// and live updates from `git-changes` and the task's review.md.
import { useEffect, useMemo, type ReactElement } from "react";
import { EmptyState, Segmented } from "@/components/ui";
import { events, type Task } from "@/lib/bindings";
import { placeNotes } from "../decorations";
import { fileRefsOf, repoNamesOf } from "../files";
import { notesForFile } from "../reviewNotes";
import { changesStore, type ChangesStore, type DiffMode } from "../store";
import { DiffEditor } from "./DiffEditor";
import { FileList } from "./FileList";

const MODES = [
  { value: "unified", label: "Unified" },
  { value: "split", label: "Side by side" },
] as const satisfies readonly { value: DiffMode; label: string }[];

export function ChangesTab({
  task,
  homeName,
  store = changesStore,
}: {
  task: Task;
  homeName?: string;
  store?: ChangesStore;
}): ReactElement {
  const st = store.store.useSelector((s) => s.byTask[task.id]);
  const mode = store.store.useSelector((s) => s.mode);
  useEffect(() => {
    void store.open(task.id);
    const offs = [
      events.gitChanges.listen((e) => {
        if (e.payload.task_id === task.id) void store.onGitChanges(task.id);
      }),
      events.taskChanged.listen((e) => {
        if (e.payload.task.id === task.id) void store.onReviewChanged(task.id);
      }),
    ];
    return () => {
      for (const off of offs) void off.then((fn) => fn());
    };
  }, [store, task.id]);

  const repos = st?.repos ?? [];
  const sel = st?.selected ?? null;
  const placed = useMemo(() => {
    if (!st?.selected || !st.diff) return [];
    const names = repoNamesOf(st.repos, homeName);
    const i = st.repos.findIndex((r) => r.repo_root === st.selected!.repo);
    if (i < 0) return [];
    const ref = fileRefsOf(st.repos, homeName).find((f) => f.repo === names[i] && f.path === st.selected!.path) ?? {
      repo: names[i],
      home: i === 0,
      path: st.selected.path,
    };
    const lines = (st.diff.status === "Deleted" ? st.diff.before : st.buffer).split("\n").length;
    return placeNotes(notesForFile(st.notes, names, ref), lines);
  }, [st, homeName]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 items-center gap-2 border-b border-line px-3 py-1">
        {st?.error && <span className="text-xs text-bad">{st.error}</span>}
        <span className="flex-1" />
        <Segmented aria-label="Diff layout" size="sm" value={mode} onValueChange={store.setMode} options={MODES} />
      </div>
      <div className="grid min-h-0 flex-1 grid-cols-[240px_minmax(0,1fr)]">
        <div className="min-h-0 border-r border-line">
          <FileList
            repos={repos}
            notes={st?.notes ?? []}
            selected={sel}
            onSelect={(s) => void store.select(task.id, s)}
            onInclude={(s) => void store.include(task.id, s)}
            {...(homeName ? { homeName } : {})}
          />
        </div>
        <div className="min-h-0">
          {sel && st?.diff ? (
            <DiffEditor
              path={`${sel.repo}/${sel.path}`}
              diff={st.diff}
              mode={mode}
              buffer={st.buffer}
              notes={placed}
              staleOnDisk={st.staleOnDisk}
              onChange={(t) => store.edit(task.id, t)}
              onSave={() => void store.save(task.id)}
              onReload={() => void store.reload(task.id)}
              onReply={(note, text) => store.reply(task.id, note, text)}
            />
          ) : (
            <EmptyState size="sm" title={repos.some((r) => r.files.length) ? "Pick a file" : "No changes yet"} />
          )}
        </div>
      </div>
    </div>
  );
}
