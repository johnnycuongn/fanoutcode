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
  KeyHint,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Textarea,
} from "@/components/ui";
import { closeNewTask, createTask, defaultProject, newTaskStore } from "../newTask";
import { JIRA_KEY_RE } from "./JiraKeyDialog";
import { useBoard } from "../store";

function Form({ onClose }: { onClose(): void }): ReactElement {
  const projects = useBoard((s) => s.projects);
  const order = useBoard((s) => s.projectOrder);
  const [intent, setIntent] = useState("");
  const [jira, setJira] = useState("");
  const [project, setProject] = useState(defaultProject() ?? "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    const key = jira.trim().toUpperCase();
    if (!intent.trim()) return setError("Describe what you want built");
    if (key && !JIRA_KEY_RE.test(key)) return setError("Use the form PROJ-123 for the Jira issue");
    if (!project) return setError("Pick a project");
    setBusy(true);
    const ok = await createTask(project, intent, key || null);
    setBusy(false);
    if (ok) onClose();
  };

  return (
    <form onSubmit={submit}>
      <DialogHeader>
        <DialogTitle>New task</DialogTitle>
        <DialogDescription>Describe it in your own words; the agent starts from the spec.</DialogDescription>
      </DialogHeader>
      <div className="flex flex-col gap-3">
        <Field label="What should be built?" error={error}>
          <Textarea
            autoFocus
            rows={4}
            value={intent}
            onChange={(e) => {
              setIntent(e.target.value);
              setError(null);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && e.metaKey) void submit(e);
            }}
          />
        </Field>
        {order.length > 1 && (
          <Field label="Project">
            <Select value={project} onValueChange={setProject}>
              <SelectTrigger aria-label="Project">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {order.map((id) => (
                  <SelectItem key={id} value={id}>
                    {projects[id]?.name ?? id}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
        )}
        <Field label="Jira issue (optional)">
          <Input mono value={jira} onChange={(e) => setJira(e.target.value)} placeholder="PROJ-123" />
        </Field>
      </div>
      <DialogFooter>
        <Button onClick={onClose}>Cancel</Button>
        <Button type="submit" variant="primary" loading={busy} trailing={<KeyHint keys="cmd+enter" size="sm" />}>
          Create task
        </Button>
      </DialogFooter>
    </form>
  );
}

/** The sheet behind ⌘T and the New task buttons; it mounts its form only while open, so each open starts clean. */
export function NewTaskDialog(): ReactElement {
  const open = newTaskStore.useSelector((s) => s.open);
  return (
    <Dialog open={open} onOpenChange={(next) => !next && closeNewTask()}>
      <DialogContent>{open && <Form onClose={closeNewTask} />}</DialogContent>
    </Dialog>
  );
}
