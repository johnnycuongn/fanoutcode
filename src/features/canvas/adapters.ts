// The only file that reads Plan 03's and Plan 05's field names: change sets, cost lines, plan steps, scopes and the
// task's repo roots become the canvas's own input types.
import type { CostSummary, PlanDoc, Project, RepoChanges, Scope, Task } from "@/lib/bindings";
import type { AgentCost, CanvasStep, FileStat, RepoRoot } from "./types";

function join(root: string, path: string): string {
  return path.startsWith("/") ? path : `${root.replace(/\/+$/, "")}/${path}`;
}

/** The change set's attributed files (`files`; `other` holds the unattributed ones) as absolute paths. */
export function fromRepoChanges(changes: RepoChanges[]): FileStat[] {
  return changes.flatMap((repo) =>
    repo.files.map((f) => ({
      path: join(repo.repo_root, f.path),
      added: f.added,
      removed: f.removed,
      owners: f.owners,
    })),
  );
}

/** One cost per `CostSummary.by_agent` line; keys are agent ids, `main` and `helpers` (A6, A19). */
export function fromCostSummary(summary: CostSummary): AgentCost[] {
  return (summary.by_agent ?? []).map((line) => ({ agentId: line.key, costUsd: line.cost_usd }));
}

export function scopeFor(task: Task): Scope {
  return { workspace: task.workspace_id, project: task.project_id, task: task.id };
}

function basename(path: string): string {
  return path.replace(/\/+$/, "").split("/").pop() ?? path;
}

/** The home project's root first, then the task's linked roots, named and coloured from their projects. */
export function rootsFor(task: Task, projects: Project[]): RepoRoot[] {
  const home = projects.find((p) => p.id === task.project_id);
  const out: RepoRoot[] = home ? [{ root: home.root, name: home.name, color: home.color }] : [];
  for (const root of task.linked_roots ?? []) {
    if (out.some((r) => r.root === root)) continue;
    const p = projects.find((x) => x.root === root);
    out.push(p ? { root, name: p.name, color: p.color } : { root, name: basename(root), color: "mute" });
  }
  return out;
}

/** A plan heading (a linked project's name) to that project's root; null for the home project or no match. */
function rootOfHeading(heading: string | null, roots: RepoRoot[]): string | null {
  if (heading === null) return null;
  const h = heading.trim().toLowerCase();
  const match = roots.find((r) => r.name.toLowerCase() === h || basename(r.root).toLowerCase() === h);
  return match && match !== roots[0] ? match.root : null;
}

/**
 * Plan 05's `PlanDoc` as canvas steps: `after` is the `(after …)` list only (the flow canvas draws the implicit
 * previous-step edge itself), owners are the Team roles naming the step, and `files` is empty because `PlanDoc`
 * carries no per-step files.
 */
export function fromPlanDoc(doc: PlanDoc, roots: RepoRoot[]): CanvasStep[] {
  return doc.steps.map((s) => ({
    number: s.number,
    title: s.title,
    done: s.done,
    after: s.explicit_deps ? s.deps : [],
    owners: doc.team.filter((t) => t.steps.includes(s.number)).map((t) => t.role),
    files: [],
    repo: rootOfHeading(s.repo, roots),
  }));
}
