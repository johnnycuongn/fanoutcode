// An in-memory DockPersist for tests.
import type { RepoCommits } from "@/lib/bindings";
import type { DockPersist, DockState } from "../types";

export const mem = (): DockPersist & { last: DockState | null } => {
  const p = {
    last: null as DockState | null,
    read: () => p.last,
    write: (s: DockState) => {
      p.last = s;
    },
  };
  return p;
};

/** Two repos of drafted commits, as `git_drafts` returns them. */
export const drafts: RepoCommits[] = [
  {
    repo_root: "/w/api",
    commits: [
      {
        message: "feat(auth): constant-time token compare\n\nCloses TC-12.",
        files: ["src/auth.ts", "src/auth.test.ts"],
      },
      { message: "chore: bump deps", files: [] },
    ],
  },
  {
    repo_root: "/w/web",
    commits: [{ message: "fix(dock): remember tab per stage", files: ["src/features/dock/store.ts"] }],
  },
];
