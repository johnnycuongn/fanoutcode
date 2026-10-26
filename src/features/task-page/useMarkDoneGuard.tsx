// "Mark done" while the task's files are still uncommitted (Fanout Code may commit off): ask first, so Done never
// hides work that is still only in the working tree.
import { useCallback, useState, type ReactNode } from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui";
import type { StageAction, Task } from "@/lib/bindings";
import { uncommittedCount } from "@/lib/uncommitted";

export function useMarkDoneGuard(task: Pick<Task, "id" | "stage"> | null): {
  /** Resolves true when `action` may go ahead. */
  guard(action: StageAction): Promise<boolean>;
  /** Runs `go` once `action` may go ahead. */
  ifAllowed(action: StageAction, go: () => Promise<unknown>): void;
  dialog: ReactNode;
} {
  const [ask, setAsk] = useState<{ count: number; resolve(ok: boolean): void } | null>(null);
  const guard = useCallback(
    async (action: StageAction) => {
      if (!task || task.stage !== "Commit" || action.type !== "Advance") return true;
      const count = await uncommittedCount(task.id);
      if (count === 0) return true;
      return new Promise<boolean>((resolve) => setAsk({ count, resolve }));
    },
    [task],
  );
  const settle = (ok: boolean) => {
    ask?.resolve(ok);
    setAsk(null);
  };
  const one = ask?.count === 1;
  const dialog = ask ? (
    <Dialog open onOpenChange={(open) => !open && settle(false)}>
      <DialogContent size="sm" hideClose>
        <DialogHeader>
          <DialogTitle>Mark done with uncommitted changes?</DialogTitle>
          <DialogDescription>
            {one ? "1 file" : `${ask.count} files`} from this task {one ? "is" : "are"} not committed yet. Commit{" "}
            {one ? "it" : "them"} from your terminal first, or mark the task done now and its card will say it is not
            committed.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button onClick={() => settle(false)}>Cancel</Button>
          <Button variant="primary" onClick={() => settle(true)}>
            Mark done anyway
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  ) : null;
  const ifAllowed = (action: StageAction, go: () => Promise<unknown>) =>
    void guard(action).then((ok) => (ok ? go() : undefined));
  return { guard, ifAllowed, dialog };
}
