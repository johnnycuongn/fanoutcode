// The Commit tab's drafts. Plan 05's commit helper writes commits.md and parses it back in Rust (`git_drafts`, the
// same parser `git_commit` uses), so the tab reads the parsed drafts instead of parsing the markdown again.
import type { RepoCommits } from "@/lib/bindings";
import { repoName } from "@/features/changes";

export interface CommitDraft {
  repo: string;
  message: string;
  files: string[];
}

export function draftsFrom(repos: RepoCommits[]): CommitDraft[] {
  return repos.flatMap((r) =>
    r.commits.map((c) => ({ repo: repoName(r.repo_root), message: c.message, files: c.files })),
  );
}

const plural = (n: number, one: string) => `${n} ${one}${n === 1 ? "" : "s"}`;

/** The confirm button: "Commit 3 files in 2 repos". */
export function commitSummary(drafts: CommitDraft[]): string {
  const files = new Set(drafts.flatMap((d) => d.files.map((f) => `${d.repo}/${f}`))).size;
  const repos = new Set(drafts.map((d) => d.repo)).size;
  return `Commit ${plural(files, "file")} in ${plural(repos, "repo")}`;
}
