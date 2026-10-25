// The merge hand-back (§6.3): while bringing the base in left conflicts, each conflicted file with the other task's
// intent (its spec summary and plan entries, from `overlap_integration_context`), Resume with instructions, Abort.
import { useEffect, useState, type ReactElement } from "react";
import { Button, EmptyState, Textarea, toast } from "@/components/ui";
import type { ConflictCtx, Task } from "@/lib/bindings";
import { errorMessage, live } from "@/lib/ipcResult";
import { MiniMarkdown } from "@/livespec/MiniMarkdown";

const ipc = live("overlapIntegrationContext", "overlapResume", "overlapAbort");

export interface IntegratePaneProps {
  task: Task;
}

/** The repo and conflicted paths when the task's merge needs you; null otherwise. */
export function mergeNeedsYou(task: Pick<Task, "state">): { repo: string; paths: string[] } | null {
  const s = task.state;
  return s.type === "NeedsYou" && s.reason.type === "MergeNeedsYou"
    ? { repo: s.reason.repo, paths: s.reason.paths }
    : null;
}

function Intent({ ctx }: { ctx: ConflictCtx }): ReactElement {
  return (
    <div className="mt-2 rounded border border-line bg-well p-2">
      <p className="text-xs font-semibold text-ink">{ctx.other_title}</p>
      {ctx.spec_excerpt && <MiniMarkdown text={ctx.spec_excerpt} className="mt-1 text-[13px]" />}
      {ctx.plan_excerpt && <MiniMarkdown text={ctx.plan_excerpt} className="mt-1 text-[13px]" />}
    </div>
  );
}

function useIntegrationContext(task: Task, active: boolean): ConflictCtx[] {
  const [ctx, setCtx] = useState<ConflictCtx[]>([]);
  useEffect(() => {
    if (!active) return;
    let on = true;
    ipc.overlapIntegrationContext(task.id).then(
      (rows) => on && setCtx(rows),
      () => on && setCtx([]),
    );
    return () => {
      on = false;
    };
  }, [task.id, active]);
  return ctx;
}

export function IntegratePane({ task }: IntegratePaneProps): ReactElement | null {
  const merge = mergeNeedsYou(task);
  const ctx = useIntegrationContext(task, merge !== null);
  const [instruction, setInstruction] = useState("");
  const [busy, setBusy] = useState(false);
  if (!merge) return null;

  const run = async (call: () => Promise<unknown>, after?: () => void) => {
    setBusy(true);
    try {
      await call();
      after?.();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  const text = instruction.trim();

  return (
    <section aria-label="Merge needs you" className="flex flex-col gap-3 p-3">
      <div>
        <h3 className="text-sm font-semibold text-ink">Merge needs you</h3>
        <p className="text-xs text-mute">
          These files still have conflicts. Tell the session how to keep both behaviours, or abort the merge.
        </p>
      </div>
      {merge.paths.length === 0 ? (
        <EmptyState size="sm" title="No conflicted files reported" />
      ) : (
        <ul className="flex flex-col gap-2">
          {merge.paths.map((path) => {
            const others = ctx.filter((c) => c.path === path);
            return (
              <li key={path} className="rounded border border-line bg-card p-2">
                <p className="font-mono text-xs text-ink">{path}</p>
                {others.length === 0 ? (
                  <p className="mt-1 text-xs text-mute">No other task&apos;s intent is recorded for this file.</p>
                ) : (
                  others.map((c) => <Intent key={`${c.path}:${c.other_title}`} ctx={c} />)
                )}
              </li>
            );
          })}
        </ul>
      )}
      <Textarea
        aria-label="Instructions for the session"
        placeholder="Continue the integration with these instructions…"
        rows={3}
        value={instruction}
        onChange={(e) => setInstruction(e.target.value)}
      />
      <div className="flex items-center justify-end gap-2">
        <Button size="sm" disabled={busy} onClick={() => void run(() => ipc.overlapAbort(task.id))}>
          Abort
        </Button>
        <Button
          size="sm"
          variant="primary"
          disabled={busy || !text}
          onClick={() =>
            void run(
              () => ipc.overlapResume(task.id, text),
              () => setInstruction(""),
            )
          }
        >
          Resume
        </Button>
      </div>
    </section>
  );
}
