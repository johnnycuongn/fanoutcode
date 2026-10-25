// Parses review.md (§8): optional `## <reviewer>` headings, and one entry per finding,
// `- <location>:<line> · <severity> · <text>`, continuation lines indented by two spaces. Plan 03's Review prompt
// asks for `<repo>/<path>` locations and the words blocker, major, minor and nit; those map onto the five
// severities here, and anything else is info.
import type { FileRef, ReviewNote, Severity } from "./types";

const ENTRY = /^\s*(?:[-*]\s+)?`?(\S+?):(\d+)\s+·\s+(\w+)\s+·\s+(.+?)`?\s*$/;
const CONTINUATION = /^ {2,}\S/;
const HEADING = /^#{2,}\s+(.*?)\s*$/;

const SEVERITY: Record<string, Severity> = {
  critical: "critical",
  high: "high",
  medium: "medium",
  low: "low",
  info: "info",
  blocker: "critical",
  major: "high",
  minor: "medium",
  nit: "low",
};

export function parseReviewNotes(markdown: string): ReviewNote[] {
  const notes: ReviewNote[] = [];
  const seen = new Map<string, number>();
  let reviewer: string | null = null;
  let current: ReviewNote | null = null;
  for (const raw of markdown.split("\n")) {
    const line = raw.trimEnd();
    const heading = HEADING.exec(line);
    if (heading) {
      reviewer = heading[1];
      current = null;
      continue;
    }
    const entry = ENTRY.exec(line);
    if (entry) {
      const [, location, lineNo, severity, text] = entry;
      const base = `${location}:${lineNo}`;
      const ordinal = seen.get(base) ?? 0;
      seen.set(base, ordinal + 1);
      current = {
        id: `${base}:${ordinal}`,
        reviewer,
        security: reviewer !== null && /security/i.test(reviewer),
        location,
        line: Number(lineNo),
        severity: SEVERITY[severity.toLowerCase()] ?? "info",
        text,
      };
      notes.push(current);
      continue;
    }
    if (current && CONTINUATION.test(raw)) current.text += `\n${raw.trim()}`;
    else current = null;
  }
  return notes;
}

/** The first segment is the repo only when it names a known repo; otherwise the location is in the home repo. */
export function splitLocation(location: string, repoNames: string[]): { repo: string | null; path: string } {
  const slash = location.indexOf("/");
  if (slash > 0) {
    const first = location.slice(0, slash);
    if (repoNames.includes(first)) return { repo: first, path: location.slice(slash + 1) };
  }
  return { repo: null, path: location };
}

function placedAt(note: ReviewNote, repoNames: string[], file: FileRef): boolean {
  const { repo, path } = splitLocation(note.location, repoNames);
  if (path !== file.path) return false;
  return repo === null ? file.home : repo === file.repo;
}

export function notesForFile(notes: ReviewNote[], repoNames: string[], file: FileRef): ReviewNote[] {
  return notes.filter((n) => placedAt(n, repoNames, file));
}

/** Notes on files outside the change set: listed on their own, never dropped (Review Focus 1). */
export function unplacedNotes(notes: ReviewNote[], repoNames: string[], files: FileRef[]): ReviewNote[] {
  return notes.filter((n) => !files.some((f) => placedAt(n, repoNames, f)));
}
