// Pure helpers for the file list: change bars, and how repos are named and referenced for review-note placement.
import type { RepoChanges } from "@/lib/bindings";
import { repoName } from "./store";
import type { FileRef } from "./types";

/** Percentages of `max` (the repo's largest added + removed); a zero max draws nothing. */
export function barWidths(added: number, removed: number, max: number): { add: number; del: number } {
  if (max <= 0) return { add: 0, del: 0 };
  return { add: Math.round((added / max) * 100), del: Math.round((removed / max) * 100) };
}

/**
 * The name review.md uses for each repo. The first entry is the task's home repo (Plan 05 lists it first); when it
 * is a worktree, `homeName` (the project folder's name, which the Review prompt writes) stands in for the
 * worktree's own folder name.
 */
export function repoNamesOf(repos: RepoChanges[], homeName?: string): string[] {
  return repos.map((r, i) => (i === 0 && homeName ? homeName : repoName(r.repo_root)));
}

export function fileRefsOf(repos: RepoChanges[], homeName?: string): FileRef[] {
  const names = repoNamesOf(repos, homeName);
  return repos.flatMap((r, i) => r.files.map((f) => ({ repo: names[i], home: i === 0, path: f.path })));
}
