// Fakes shared by the Changes store and component tests.
import { vi } from "vitest";
import type { ChangedFile, RepoChanges } from "@/lib/bindings";
import type { ChangesIpc } from "../store";

export const file = (path: string, added = 1, removed = 0): ChangedFile => ({
  path,
  status: "Modified",
  added,
  removed,
  binary: false,
  owners: [],
  shared: false,
  source: "Hook",
});

export const repos = (): RepoChanges[] => [
  { repo_root: "/w/api", base: "abc", files: [file("src/a.ts", 3, 1)], other: [file("src/stray.ts")] },
  { repo_root: "/w/web", base: "abc", files: [file("src/b.ts")], other: [] },
];

export function fakeIpc(): ChangesIpc & { disk: string } {
  const f = {
    disk: "old",
    gitChanges: vi.fn(async () => repos()),
    gitDiff: vi.fn(async (_t: string, repo: string, path: string) => ({
      repo_root: repo,
      path,
      status: "Modified",
      before: "base",
      after: f.disk,
      binary: false,
    })),
    gitFileWrite: vi.fn(async (_t: string, _r: string, _p: string, c: string) => {
      f.disk = c;
      return null;
    }),
    gitIncludeOther: vi.fn(async () => null),
    memoryRead: vi.fn(async () => "- api/src/a.ts:1 · high · x"),
    tasksAction: vi.fn(async () => {}),
  };
  return f as unknown as ChangesIpc & { disk: string };
}
