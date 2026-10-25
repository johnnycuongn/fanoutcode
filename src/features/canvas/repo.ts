// Which repo a path belongs to, and whether cards carry repo badges (a workspace task over two or more roots, §10.7).
import type { Task } from "@/lib/bindings";
import type { RepoRoot } from "./types";

/** The longest root that is a prefix of `path` on a `/` boundary. */
export function repoFor(path: string, roots: RepoRoot[]): RepoRoot | null {
  let best: RepoRoot | null = null;
  for (const r of roots) {
    const root = r.root.replace(/\/+$/, "");
    if ((path === root || path.startsWith(root + "/")) && (!best || root.length > best.root.length)) best = r;
  }
  return best;
}

export function showRepoBadges(task: Pick<Task, "workspace_id">, roots: RepoRoot[]): boolean {
  return task.workspace_id !== null && roots.length >= 2;
}
