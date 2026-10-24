// The Ticket section: tasks linked to a Jira issue, and a search over the project's Jira (debounced 300 ms).
import { useEffect, useState, type ReactElement } from "react";
import { Badge, Button, Input } from "@/components/ui";
import type { ProjectId } from "@/lib/bindings";
import { jiraSearch, type Issue } from "@/lib/pendingCommands";
import { selectTaskList, tasksStore } from "@/lib/stores/tasksStore";
import { openTaskInCurrentView } from "@/features/shell";

export const JIRA_SEARCH_DEBOUNCE_MS = 300;

function useJiraSearch(projectId: ProjectId | undefined, query: string): Issue[] {
  const [results, setResults] = useState<Issue[]>([]);
  useEffect(() => {
    const q = query.trim();
    if (!projectId || !q) return;
    let live = true;
    const t = setTimeout(() => {
      void jiraSearch(projectId, q).then((r) => live && r?.status === "ok" && setResults(r.data));
    }, JIRA_SEARCH_DEBOUNCE_MS);
    return () => {
      live = false;
      clearTimeout(t);
    };
  }, [projectId, query]);
  return query.trim() ? results : [];
}

export function TicketSection({ projectId }: { projectId?: ProjectId }): ReactElement {
  const all = tasksStore.useSelector(selectTaskList);
  const [query, setQuery] = useState("");
  const results = useJiraSearch(projectId, query);
  const linked = all.filter((t) => t.jira_key && !t.archived && (!projectId || t.project_id === projectId));
  return (
    <div className="flex flex-col gap-2 px-2">
      <h3 className="py-1 text-xs font-medium text-mute">Linked issues</h3>
      {linked.length === 0 && <p className="text-xs text-mute">No task is linked to an issue.</p>}
      {linked.map((t) => (
        <Button
          key={t.id}
          variant="ghost"
          size="sm"
          className="w-full justify-start gap-2"
          onClick={() => openTaskInCurrentView(t.id, { dockTab: "ticket" })}
        >
          <Badge size="sm" mono>
            {t.jira_key}
          </Badge>
          <span className="truncate">{t.title}</span>
        </Button>
      ))}
      <Input
        size="sm"
        aria-label="Search Jira"
        placeholder="Search Jira…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      <ul className="flex flex-col gap-1">
        {results.map((issue) => (
          <li key={issue.key} className="flex items-center gap-2 text-sm">
            <Badge size="sm" mono>
              {issue.key}
            </Badge>
            <span className="truncate text-ink">{issue.summary}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
