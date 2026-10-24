import { useState, type FormEvent, type ReactElement } from "react";
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
import { updateTask } from "../actions";

export const JIRA_KEY_RE = /^[A-Z][A-Z0-9_]*-\d+$/;

/** Link, change or remove the task's Jira issue key. */
export function JiraKeyDialog({
  task,
  open,
  onClose,
}: {
  task: Pick<Task, "id" | "jira_key">;
  open: boolean;
  onClose(): void;
}): ReactElement {
  const [value, setValue] = useState(task.jira_key ?? "");
  const [error, setError] = useState<string | null>(null);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    const key = value.trim().toUpperCase();
    if (!JIRA_KEY_RE.test(key)) {
      setError("Use the form PROJ-123");
      return;
    }
    void updateTask(task.id, { jira_key: key });
    onClose();
  };
  const unlink = () => {
    void updateTask(task.id, { jira_key: null });
    onClose();
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent size="sm">
        <form onSubmit={submit}>
          <DialogHeader>
            <DialogTitle>{task.jira_key ? "Change Jira issue" : "Link a Jira issue"}</DialogTitle>
            <DialogDescription>The issue key, for example PROJ-123.</DialogDescription>
          </DialogHeader>
          <Field label="Issue key" error={error}>
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
            {task.jira_key && <Button onClick={unlink}>Unlink</Button>}
            <Button type="submit" variant="primary">
              Link
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
