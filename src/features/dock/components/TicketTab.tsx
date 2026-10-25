// The Ticket tab (§6.5): the task's Jira issue (summary, status, assignee, a link out and a comment box), or, with no
// issue linked, a key field and a search to link one. Errors show verbatim.
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState, type ReactElement } from "react";
import { Avatar, Button, Chip, EmptyState, Field, Input, Textarea } from "@/components/ui";
import type { Task } from "@/lib/bindings";
import { ticketStore, type TicketStore } from "../ticketStore";

function LinkForm({ task, store }: { task: Task; store: TicketStore }) {
  const [key, setKey] = useState("");
  const [query, setQuery] = useState("");
  const results = store.store.useSelector((s) => s[task.id]?.results);
  return (
    <div className="grid max-w-md gap-3">
      <form
        className="flex items-end gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (key.trim()) void store.link(task, key.trim().toUpperCase());
        }}
      >
        <Field label="Issue key" className="flex-1">
          <Input size="sm" mono placeholder="TC-12" value={key} onChange={(e) => setKey(e.target.value)} />
        </Field>
        <Button size="sm" type="submit" disabled={!key.trim()}>
          Link
        </Button>
      </form>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (query.trim()) void store.search(task, query.trim());
        }}
      >
        <Input
          size="sm"
          aria-label="Search Jira"
          placeholder="Search issues"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </form>
      {results && results.length > 0 && (
        <ul className="grid gap-1 text-[13px]">
          {results.map((r) => (
            <li key={r.key} className="flex items-center gap-2">
              <span className="font-mono text-mute">{r.key}</span>
              <span className="min-w-0 flex-1 truncate">{r.summary}</span>
              <Button
                size="sm"
                variant="secondary"
                aria-label={`Link ${r.key}`}
                onClick={() => void store.link(task, r.key)}
              >
                Link
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function TicketTab({ task, store = ticketStore }: { task: Task; store?: TicketStore }): ReactElement {
  const st = store.store.useSelector((s) => s[task.id]);
  const [body, setBody] = useState("");
  useEffect(() => {
    void store.load(task);
    // reload when the linked key changes, not on every task-changed
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [store, task.id, task.jira_key]);

  const issue = st?.issue ?? null;
  return (
    <div className="h-full overflow-auto px-4 py-3">
      {!task.jira_key && <LinkForm task={task} store={store} />}
      {task.jira_key && !issue && !st?.error && <EmptyState size="sm" title={`Loading ${task.jira_key}…`} />}
      {issue && (
        <div className="grid max-w-2xl gap-3">
          <div className="flex items-center gap-2">
            <span className="font-mono text-xs text-mute">{issue.key}</span>
            <h3 className="min-w-0 flex-1 truncate font-semibold">{issue.summary}</h3>
            <Chip size="sm" tone="accent">
              {issue.status}
            </Chip>
          </div>
          <div className="flex items-center gap-2 text-xs text-mute">
            {issue.assignee ? (
              <>
                <Avatar size="sm" name={issue.assignee} />
                <span>{issue.assignee}</span>
              </>
            ) : (
              <span>Unassigned</span>
            )}
            <span className="flex-1" />
            <Button size="sm" variant="secondary" onClick={() => void openUrl(issue.url).catch(() => {})}>
              Open in Jira
            </Button>
          </div>
          <form
            className="grid gap-1.5"
            onSubmit={(e) => {
              e.preventDefault();
              void store.comment(task, body.trim()).then((ok) => ok && setBody(""));
            }}
          >
            <Textarea
              size="sm"
              autoGrow
              aria-label="Comment"
              placeholder="Comment on the issue"
              value={body}
              onChange={(e) => setBody(e.target.value)}
            />
            <div>
              <Button size="sm" type="submit" disabled={!body.trim()} loading={st?.posting ?? false}>
                Post comment
              </Button>
            </div>
          </form>
        </div>
      )}
      {st?.error && (
        <p role="alert" className="mt-3 text-[13px] text-bad">
          {st.error}
        </p>
      )}
    </div>
  );
}
