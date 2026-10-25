// Search the project's Jira and start a task from an issue.
import { useState, type ReactElement } from "react";
import { Button, Chip, Input, toast } from "@/components/ui";
import type { ProjectId, Task, WorkspaceId } from "@/lib/bindings";
import { jiraStore } from "../jiraStore";
import { messageOf } from "../result";

export type IssuePickerProps = { projectId: ProjectId; workspaceId?: WorkspaceId | null; onCreated?(task: Task): void };

export function IssuePicker({ projectId, workspaceId, onCreated }: IssuePickerProps): ReactElement {
  const search = jiraStore.useSelector((s) => s.search);
  const [query, setQuery] = useState("");
  const [creating, setCreating] = useState<string | null>(null);
  const change = (q: string) => {
    setQuery(q);
    void jiraStore.search(projectId, q);
  };
  const create = async (key: string) => {
    setCreating(key);
    try {
      onCreated?.(await jiraStore.createTask(projectId, key, workspaceId));
    } catch (e) {
      toast.error(messageOf(e));
    } finally {
      setCreating(null);
    }
  };
  const results = query.trim() && search.query === query ? search.results : [];
  return (
    <div className="flex flex-col gap-2">
      <Input
        size="sm"
        aria-label="Search Jira"
        placeholder="Key or text"
        value={query}
        onChange={(e) => change(e.target.value)}
      />
      {search.pending && query.trim() && <p className="text-xs text-mute">Searching…</p>}
      <ul className="flex flex-col gap-1" aria-label="Issues">
        {results.map((i) => (
          <li key={i.key} className="flex items-center gap-2 text-sm">
            <span className="font-mono text-xs text-mute">{i.key}</span>
            <span className="min-w-0 flex-1 truncate text-ink">{i.summary}</span>
            <Chip size="sm">{i.status}</Chip>
            <Button size="sm" variant="secondary" loading={creating === i.key} onClick={() => void create(i.key)}>
              Create task
            </Button>
          </li>
        ))}
      </ul>
    </div>
  );
}
