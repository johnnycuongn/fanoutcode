// The Changes tab's left column: each repo's changed files with change bars and note counts, the repo's "Other
// changes" (seen in the tree, not attributed to the task) with an Include control, and the notes on files outside
// the change set, which are listed rather than dropped (Review Focus 1).
import { useState, type ReactElement } from "react";
import { Badge, Button, Chip, cx } from "@/components/ui";
import type { ChangedFile, FileStatus, RepoChanges } from "@/lib/bindings";
import { barWidths, fileRefsOf, repoNamesOf } from "../files";
import { notesForFile, unplacedNotes } from "../reviewNotes";
import type { Selection } from "../store";
import type { ReviewNote } from "../types";

export interface FileListProps {
  repos: RepoChanges[];
  notes: ReviewNote[];
  selected: Selection | null;
  onSelect(s: Selection): void;
  onInclude(s: Selection): void;
  homeName?: string;
}

const GLYPH: Record<FileStatus, string> = { Added: "A", Modified: "M", Deleted: "D", Renamed: "R", Untracked: "U" };

function Row(p: {
  file: ChangedFile;
  max: number;
  notes: ReviewNote[];
  selected: boolean;
  onSelect(): void;
}): ReactElement {
  const { add, del } = barWidths(p.file.added, p.file.removed, p.max);
  const security = p.notes.some((n) => n.security);
  return (
    <li
      role="option"
      aria-selected={p.selected}
      tabIndex={0}
      onClick={p.onSelect}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          p.onSelect();
        }
      }}
      className={cx(
        "flex cursor-pointer items-center gap-1.5 rounded px-2 py-1 text-xs outline-none hover:bg-well",
        "focus-visible:ring-2 focus-visible:ring-accent/60",
        p.selected && "bg-well text-ink",
      )}
    >
      <span aria-hidden className="w-3 font-mono text-mute">
        {GLYPH[p.file.status]}
      </span>
      <span className="min-w-0 flex-1 truncate font-mono" title={p.file.path}>
        {p.file.path}
      </span>
      {security && (
        <Chip size="sm" tone="bad">
          security
        </Chip>
      )}
      {p.notes.length > 0 && (
        <Badge size="sm" tone="warn">
          {p.notes.length}
        </Badge>
      )}
      <span data-bar aria-hidden className="flex h-1.5 w-10 overflow-hidden rounded-full bg-well">
        <span className="h-full bg-ok" style={{ width: `${add}%` }} />
        <span className="h-full bg-bad" style={{ width: `${del}%` }} />
      </span>
    </li>
  );
}

function Repo(p: { repo: RepoChanges; name: string; home: boolean; props: FileListProps; names: string[] }) {
  const { repo, name, props } = p;
  const [showOther, setShowOther] = useState(false);
  const max = Math.max(0, ...repo.files.map((f) => f.added + f.removed));
  const plus = repo.files.reduce((n, f) => n + f.added, 0);
  const minus = repo.files.reduce((n, f) => n + f.removed, 0);
  return (
    <section className="mb-3">
      <div className="flex items-baseline gap-2 px-2 py-1">
        <h3 className="text-xs font-semibold text-ink">{name}</h3>
        <span className="text-[11px] text-mute tabular-nums">
          +{plus} −{minus}
        </span>
      </div>
      <ul role="listbox" aria-label={`${name} changes`} className="grid gap-px">
        {repo.files.map((f) => (
          <Row
            key={f.path}
            file={f}
            max={max}
            notes={notesForFile(props.notes, p.names, { repo: name, home: p.home, path: f.path })}
            selected={props.selected?.repo === repo.repo_root && props.selected.path === f.path}
            onSelect={() => props.onSelect({ repo: repo.repo_root, path: f.path })}
          />
        ))}
      </ul>
      {repo.other.length > 0 && (
        <div className="px-2 pt-1">
          <Button variant="link" size="sm" onClick={() => setShowOther((v) => !v)}>
            {showOther ? "Hide other changes" : "Show other changes"}
          </Button>
          {showOther && (
            <ul className="grid gap-px pt-1">
              {repo.other.map((f) => (
                <li key={f.path} className="flex items-center gap-2 text-xs text-mute">
                  <span className="min-w-0 flex-1 truncate font-mono">{f.path}</span>
                  <Button
                    variant="secondary"
                    size="sm"
                    aria-label={`Include ${f.path}`}
                    onClick={() => props.onInclude({ repo: repo.repo_root, path: f.path })}
                  >
                    Include
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}

export function FileList(props: FileListProps): ReactElement {
  const names = repoNamesOf(props.repos, props.homeName);
  const unplaced = unplacedNotes(props.notes, names, fileRefsOf(props.repos, props.homeName));
  return (
    <nav aria-label="Changed files" className="h-full overflow-auto py-2">
      {props.repos.map((r, i) => (
        <Repo key={r.repo_root} repo={r} name={names[i]} home={i === 0} props={props} names={names} />
      ))}
      {unplaced.length > 0 && (
        <section className="border-t border-line px-2 pt-2">
          <h4 className="mb-1 text-[11px] font-semibold text-mute">Notes on files outside the change set</h4>
          <ul className="grid gap-1 text-xs">
            {unplaced.map((n) => (
              <li key={n.id} className="grid">
                <span className="font-mono text-ink">{`${n.location}:${n.line}`}</span>
                <span className="text-mute">{n.text}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
    </nav>
  );
}
