// The dock's per-window state: its position, the tab and height remembered per stage (persisted), and the spec
// watcher's log per task (not persisted).
import type { DockPosition, Stage, TaskId } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { DEFAULT_HEIGHT, GROWN_HEIGHT, defaultTabFor, entryState, resolveTab, visibleTabs } from "./tabRules";
import type { DockPersist, DockState, DockTab, WatcherEntry } from "./types";

const KEY = "teamcode.dock";
const WATCHER_CAP = 50;

export const localPersist: DockPersist = {
  read() {
    try {
      const raw = globalThis.localStorage?.getItem(KEY);
      return raw ? (JSON.parse(raw) as Partial<DockState>) : null;
    } catch {
      return null;
    }
  },
  write(s) {
    try {
      const { position, tabByStage, heightByStage } = s;
      globalThis.localStorage?.setItem(KEY, JSON.stringify({ position, tabByStage, heightByStage }));
    } catch {
      // storage full or blocked: the dock still works for this session
    }
  },
};

export function createDockStore(persist: DockPersist = localPersist) {
  const saved = persist.read() ?? {};
  const store = createStore<DockState>({
    position: saved.position ?? "Bottom",
    tabByStage: saved.tabByStage ?? {},
    heightByStage: saved.heightByStage ?? {},
    watcherLog: {},
  });
  const update = (fn: (s: DockState) => DockState) => {
    store.set(fn);
    persist.write(store.get());
  };
  const heightFor = (stage: Stage) => store.get().heightByStage[stage] ?? DEFAULT_HEIGHT;
  const setHeight = (stage: Stage, h: number) =>
    update((s) => ({ ...s, heightByStage: { ...s.heightByStage, [stage]: h } }));
  const selectTab = (stage: Stage, tab: DockTab) =>
    update((s) => ({ ...s, tabByStage: { ...s.tabByStage, [stage]: tab } }));

  return {
    store,
    setPosition: (position: DockPosition) => update((s) => ({ ...s, position })),
    defaultOpen: (stage: Stage) => stage !== "Spec",
    tabFor: (stage: Stage, hasTestCommand: boolean): DockTab =>
      resolveTab(stage, store.get().tabByStage[stage], visibleTabs(stage, hasTestCommand)),
    selectTab,
    heightFor,
    isGrown: (stage: Stage) => heightFor(stage) >= GROWN_HEIGHT,
    setHeight,
    toggleGrow: (stage: Stage) => setHeight(stage, heightFor(stage) >= GROWN_HEIGHT ? DEFAULT_HEIGHT : GROWN_HEIGHT),
    enterStage(stage: Stage): { open: boolean | null; grown: boolean | null; tab: DockTab } {
      const e = entryState(stage);
      const tab = defaultTabFor(stage);
      update((s) => ({
        ...s,
        tabByStage: { ...s.tabByStage, [stage]: tab },
        heightByStage:
          e.grown === null ? s.heightByStage : { ...s.heightByStage, [stage]: e.grown ? GROWN_HEIGHT : DEFAULT_HEIGHT },
      }));
      return { ...e, tab };
    },
    addWatcherEntry(taskId: TaskId, entry: WatcherEntry): void {
      store.set((s) => ({
        ...s,
        watcherLog: { ...s.watcherLog, [taskId]: [entry, ...(s.watcherLog[taskId] ?? [])].slice(0, WATCHER_CAP) },
      }));
    },
  };
}

export type DockStore = ReturnType<typeof createDockStore>;

export const dockStore = createDockStore();
