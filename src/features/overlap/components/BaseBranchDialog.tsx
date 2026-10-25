// The base branch, confirmed the first time "Bring <base> in" is used on a task (§6.3): prefilled from the repo's
// default branch, editable, stored on the task by `overlap_set_base`, which rejects an unsafe name as Invalid.
import { useEffect, useState, type FormEvent, type ReactElement } from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Field,
  Input,
} from "@/components/ui";
import type { Task } from "@/lib/bindings";
import { errorMessage, live } from "@/lib/ipcResult";

const ipc = live("overlapBaseSuggestion", "overlapSetBase");

export interface BaseBranchDialogProps {
  task: Pick<Task, "id" | "title">;
  open: boolean;
  onClose(): void;
  /** The task as stored with its base; the caller integrates next. */
  onConfirmed(task: Task): void;
}

export function BaseBranchDialog({ task, open, onClose, onConfirmed }: BaseBranchDialogProps): ReactElement {
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    let live = true;
    ipc.overlapBaseSuggestion(task.id).then(
      (suggestion) => live && setValue(suggestion ?? ""),
      () => live && setValue(""),
    );
    return () => {
      live = false;
    };
  }, [open, task.id]);

  const close = () => {
    setError(null);
    onClose();
  };
  const branch = value.trim();
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!branch || busy) return;
    setBusy(true);
    try {
      onConfirmed(await ipc.overlapSetBase(task.id, branch));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && close()}>
      <DialogContent size="sm">
        <form onSubmit={(e) => void submit(e)}>
          <DialogHeader>
            <DialogTitle>Which branch is the base?</DialogTitle>
            <DialogDescription>
              The session merges this branch into {task.title}&apos;s branch. It is stored on the task.
            </DialogDescription>
          </DialogHeader>
          <Field label="Base branch" error={error}>
            <Input
              mono
              autoFocus
              value={value}
              onChange={(e) => {
                setValue(e.target.value);
                setError(null);
              }}
            />
          </Field>
          <DialogFooter>
            <Button onClick={close}>Cancel</Button>
            <Button type="submit" variant="primary" disabled={!branch || busy}>
              Bring {branch || "base"} in
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
