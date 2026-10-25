// The Costs panel's data: the summary at a scope, the grouping, and whether cost collection is on. Reloads on
// `cost-updated`, debounced one second.
import { commands, events, type CostSummary, type Scope } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import type { CostGroup } from "./costFormat";
import { unwrap } from "./result";

export type CostsState = { scope: Scope | null; summary: CostSummary | null; group: CostGroup; enabled: boolean };

const base = createStore<CostsState>({ scope: null, summary: null, group: "project", enabled: true });
const RELOAD_DEBOUNCE_MS = 1000;
let reloadTimer: ReturnType<typeof setTimeout> | null = null;
let unlisten: Promise<() => void> | null = null;

const full = (s: Partial<Scope>): Scope => ({
  workspace: s.workspace ?? null,
  project: s.project ?? null,
  task: s.task ?? null,
});

async function fetchSummary(scope: Scope) {
  const summary = unwrap(await commands.costSummary(scope));
  if (base.get().scope === scope) base.set({ summary });
}

function listen() {
  unlisten ??= events.costUpdated.listen(() => {
    if (reloadTimer) clearTimeout(reloadTimer);
    reloadTimer = setTimeout(() => {
      reloadTimer = null;
      const scope = base.get().scope;
      if (scope) void fetchSummary(scope).catch(() => undefined);
    }, RELOAD_DEBOUNCE_MS);
  });
}

export const costsStore = {
  ...base,
  async load(scope: Partial<Scope>): Promise<void> {
    const s = full(scope);
    base.set({ scope: s, summary: null, group: s.workspace && !s.project ? "project" : "task" });
    listen();
    const [settings] = await Promise.all([
      commands.settingsGet({ workspace: null, project: null, task: null }),
      fetchSummary(s),
    ]);
    if (settings.status === "ok") base.set({ enabled: settings.data.cost_overview });
  },
  setGroup(group: CostGroup) {
    base.set({ group });
  },
  reset() {
    if (reloadTimer) clearTimeout(reloadTimer);
    reloadTimer = null;
    base.reset();
  },
};
