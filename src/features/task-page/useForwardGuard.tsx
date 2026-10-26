// The forward step asks first when it would hide something: "Mark done" while the task's files are still uncommitted
// (Fanout Code may commit off), and "Approve changes" while review.md lists blockers. The blocker count also shows on
// the button before it is pressed.
import { useCallback, useEffect, useState, type ReactNode } from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui";
import { commands, events, type StageAction, type Task } from "@/lib/bindings";
import { uncommittedCount } from "@/lib/uncommitted";
import { parseReviewNotes } from "@/features/changes";

interface Ask {
  title: string;
  body: string;
  confirm: string;
  resolve(ok: boolean): void;
}

const plural = (n: number, one: string, many: string) => (n === 1 ? one : many);

/** The blocker findings ("critical") in the task's review.md; [] when there is none or it cannot be read. */
async function reviewBlockers(taskId: string): Promise<string[]> {
  try {
    const r = await commands.memoryRead(taskId, "review.md");
    const md = r?.status === "ok" ? r.data : null;
    return md
      ? parseReviewNotes(md)
          .filter((n) => n.severity === "critical")
          .map((n) => n.text)
      : [];
  } catch {
    return [];
  }
}

/** The blockers review.md lists while the task is in Review, kept fresh as the task changes. */
function useBlockers(task: Pick<Task, "id" | "stage"> | null): string[] {
  const [blockers, setBlockers] = useState<string[]>([]);
  const id = task?.id ?? null;
  const inReview = task?.stage === "Review";
  useEffect(() => {
    if (!id || !inReview) return;
    let live = true;
    const load = () => void reviewBlockers(id).then((b) => live && setBlockers(b));
    load();
    const off = events.taskChanged
      .listen((e) => {
        if (e.payload.task.id === id) load();
      })
      .catch(() => () => {});
    return () => {
      live = false;
      void off.then((fn) => fn());
    };
  }, [id, inReview]);
  return inReview ? blockers : [];
}

export function useForwardGuard(task: Pick<Task, "id" | "stage"> | null): {
  /** Blockers review.md lists, while in Review. */
  blockers: number;
  /** Resolves true when `action` may go ahead. */
  guard(action: StageAction): Promise<boolean>;
  /** Runs `go` once `action` may go ahead. */
  ifAllowed(action: StageAction, go: () => Promise<unknown>): void;
  dialog: ReactNode;
} {
  const [ask, setAsk] = useState<Ask | null>(null);
  const blockers = useBlockers(task);
  const confirmWith = (a: Omit<Ask, "resolve">) => new Promise<boolean>((resolve) => setAsk({ ...a, resolve }));
  const guard = useCallback(
    async (action: StageAction) => {
      if (!task) return true;
      if (task.stage === "Commit" && action.type === "Advance") {
        const n = await uncommittedCount(task.id);
        if (n === 0) return true;
        return confirmWith({
          title: "Mark done with uncommitted changes?",
          body: `${n} ${plural(n, "file", "files")} from this task ${plural(n, "is", "are")} not committed yet. Commit ${plural(n, "it", "them")} from your terminal first, or mark the task done now and its card will say it is not committed.`,
          confirm: "Mark done anyway",
        });
      }
      if (task.stage === "Review" && action.type === "ApproveChanges") {
        const found = await reviewBlockers(task.id);
        if (found.length === 0) return true;
        const n = found.length;
        return confirmWith({
          title: `Approve with ${n} ${plural(n, "blocker", "blockers")} in review?`,
          body: `review.md lists ${n} ${plural(n, "blocker", "blockers")}, starting with: ${found[0]} Ask the agent to fix ${plural(n, "it", "them")} first, or approve anyway.`,
          confirm: "Approve anyway",
        });
      }
      return true;
    },
    [task],
  );
  const settle = (ok: boolean) => {
    ask?.resolve(ok);
    setAsk(null);
  };
  const ifAllowed = (action: StageAction, go: () => Promise<unknown>) =>
    void guard(action).then((ok) => (ok ? go() : undefined));
  const dialog = ask ? (
    <Dialog open onOpenChange={(open) => !open && settle(false)}>
      <DialogContent size="sm" hideClose>
        <DialogHeader>
          <DialogTitle>{ask.title}</DialogTitle>
          <DialogDescription>{ask.body}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button onClick={() => settle(false)}>Cancel</Button>
          <Button variant="primary" onClick={() => settle(true)}>
            {ask.confirm}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  ) : null;
  return { blockers: blockers.length, guard, ifAllowed, dialog };
}
