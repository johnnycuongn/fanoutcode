// The projects of this window's container, in the window's order (a workspace's project order). Fed by
// `projects_list`; `upsertProject` records a project after `projects_update`.
import { commands, type Project, type ProjectId } from "@/lib/bindings";
import { createStore } from "./createStore";

export type ProjectsState = { byId: Record<ProjectId, Project>; order: ProjectId[] };

export const projectsStore = createStore<ProjectsState>({ byId: {}, order: [] });

/** Loads the projects; with `ids` (the window's `project_ids`) keeps only those, in that order. */
export async function loadProjects(ids?: ProjectId[]): Promise<void> {
  const r = await commands.projectsList();
  if (r.status !== "ok") return;
  const all: Record<ProjectId, Project> = {};
  for (const p of r.data) all[p.id] = p;
  const order = ids ? ids.filter((id) => all[id]) : r.data.map((p) => p.id);
  const byId: Record<ProjectId, Project> = {};
  for (const id of order) byId[id] = all[id];
  projectsStore.set({ byId, order });
}

export function selectProject(id: ProjectId): Project | undefined {
  return projectsStore.get().byId[id];
}

export function upsertProject(project: Project): void {
  projectsStore.set((s) => ({
    byId: { ...s.byId, [project.id]: project },
    order: s.order.includes(project.id) ? s.order : [...s.order, project.id],
  }));
}

/** The window's projects in order (a stable array per state, for `useSelector`). */
let listCache: { state: ProjectsState; list: Project[] } | null = null;
export function selectProjectList(s: ProjectsState): Project[] {
  if (listCache?.state === s) return listCache.list;
  const list = s.order.map((id) => s.byId[id]).filter((p): p is Project => Boolean(p));
  listCache = { state: s, list };
  return list;
}
