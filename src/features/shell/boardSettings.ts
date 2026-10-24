// The resolved settings the Board takes (Plan 07's BoardSettings), from the window's scope and its projects' scopes.
import type { Container, DetailLevel, ProjectId } from "@/lib/bindings";
import type { BoardSettings } from "@/features/board";
import { appScope, projectScope, scopeFor, scopeKey, type Settings } from "@/features/settings";

export function boardSettings(
  container: Container,
  byScope: Record<string, Settings>,
  projectIds: ProjectId[],
): BoardSettings {
  const own = byScope[scopeKey(scopeFor(container))];
  const app = byScope[scopeKey(appScope())];
  const detailLevelByProject: Record<string, DetailLevel> = {};
  for (const id of projectIds) {
    const level = byScope[scopeKey(projectScope(id))]?.detail_level;
    if (level) detailLevelByProject[id] = level;
  }
  return {
    detailLevel: own?.detail_level ?? app?.detail_level ?? "Simple",
    detailLevelByProject,
    costOverview: app?.cost_overview ?? own?.cost_overview ?? false,
    workspaceBoard: own?.workspace_board ?? "Swimlanes",
    projectLayer: own?.workspace_project_layer ?? "MultiRoot",
  };
}
