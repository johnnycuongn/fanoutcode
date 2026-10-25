// The Commit tab (§6.6): the drafted messages per repo with their files and a Copy button; the Commit button only
// when "Fanout Code may commit" is on, behind a confirmation; Draft again after a failed draft. The app never commits on
// its own.
import { useEffect, useState, type ReactElement } from "react";
import {
  Button,
  Chip,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  EmptyState,
} from "@/components/ui";
import { events, type Task } from "@/lib/bindings";
import type { TaskSettings } from "@/features/task-page";
import { commitSummary, type CommitDraft } from "../commits";
import { commitStore, type CommitStore } from "../commitStore";

const EMPTY: CommitDraft[] = [];
const OFF_HINT = "Fanout Code may commit is off for this project; commit from your own terminal.";

export function CommitTab({
  task,
  settings,
  store = commitStore,
}: {
  task: Task;
  settings: TaskSettings;
  store?: CommitStore;
}): ReactElement {
  const drafts = store.store.useSelector((s) => s[task.id]?.drafts ?? EMPTY);
  const error = store.store.useSelector((s) => s[task.id]?.error ?? null);
  const [confirming, setConfirming] = useState(false);
  useEffect(() => {
    void store.load(task.id);
    // the helper writes commits.md when the task enters Commit
    const off = events.taskChanged.listen((e) => {
      if (e.payload.task.id === task.id) void store.load(task.id);
    });
    return () => void off.then((fn) => fn());
  }, [store, task.id]);

  const repos = [...new Set(drafts.map((d) => d.repo))];
  const drafting = task.state.type === "Working";
  const failure =
    task.state.type === "NeedsYou" && task.state.reason.type === "Error" ? task.state.reason.message : null;
  return (
    <div className="h-full overflow-auto px-4 py-3">
      <div className="mb-3 flex items-center gap-2">
        {settings.mayCommit ? (
          <Button size="sm" variant="primary" disabled={drafts.length === 0} onClick={() => setConfirming(true)}>
            Commit
          </Button>
        ) : (
          <p className="text-xs text-mute">{OFF_HINT}</p>
        )}
        <span className="flex-1" />
        <Button size="sm" variant="secondary" disabled={drafting} onClick={() => void store.redraft(task.id)}>
          Draft again
        </Button>
      </div>
      {failure && (
        <p role="alert" className="mb-2 text-[13px] text-bad">
          {failure}
        </p>
      )}
      {error && (
        <p role="alert" className="mb-2 text-[13px] text-bad">
          {error}
        </p>
      )}
      {drafts.length === 0 && <EmptyState size="sm" title="No drafted commits yet" />}
      {repos.map((repo) => (
        <section key={repo} className="mb-4">
          <h3 className="mb-1 text-xs font-semibold text-ink">{repo}</h3>
          {drafts
            .filter((d) => d.repo === repo)
            .map((d, i) => (
              <div key={i} className="mb-2 rounded-card border border-line bg-card p-2">
                <pre className="mb-1.5 font-mono text-[12px] whitespace-pre-wrap">{d.message}</pre>
                <div className="flex flex-wrap items-center gap-1">
                  {d.files.map((f) => (
                    <Chip key={f} size="sm">
                      {f}
                    </Chip>
                  ))}
                  <span className="flex-1" />
                  <Button size="sm" variant="secondary" onClick={() => void store.copy(task.id, d)}>
                    Copy
                  </Button>
                </div>
              </div>
            ))}
        </section>
      ))}
      <Dialog open={confirming} onOpenChange={setConfirming}>
        <DialogContent size="sm">
          <DialogHeader>
            <DialogTitle>Commit the drafts?</DialogTitle>
            <DialogDescription>Each draft is committed with only its own files.</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button size="sm" variant="ghost" onClick={() => setConfirming(false)}>
              Cancel
            </Button>
            <Button
              size="sm"
              variant="primary"
              onClick={() => {
                setConfirming(false);
                void store.commit(task.id);
              }}
            >
              {commitSummary(drafts)}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
