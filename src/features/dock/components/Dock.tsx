// The dock (⌘J): a bottom panel or a right inspector holding the stage's tabs (§6, §10.3). The tab and height are
// remembered per stage in the dock store; whether it is open is the pane's own state.
import { useEffect, useState, type ReactElement, type ReactNode } from "react";
import { EmptyState, Panel, Tabs, TabsContent } from "@/components/ui";
import type { Project, Task } from "@/lib/bindings";
import { ChangesTab, changesStore, repoName } from "@/features/changes";
import { mergeNeedsYou, OverlapTab, overlapStore, refreshOverlaps } from "@/features/overlap";
import type { TaskSettings } from "@/features/task-page";
import { dockStore, type DockStore } from "../store";
import { GROWN_HEIGHT, resolveTab, visibleTabs } from "../tabRules";
import type { DockTab } from "../types";
import { CommitTab } from "./CommitTab";
import { DockTabStrip } from "./DockTabStrip";
import { MemoryTab } from "./MemoryTab";
import { TestTab } from "./TestTab";
import { TicketTab } from "./TicketTab";

export interface DockProps {
  task: Task;
  settings: TaskSettings;
  open: boolean;
  teamSlot: ReactNode;
  store?: DockStore;
  onOpenTicket?: () => void;
  projects?: Project[];
}

function body(tab: DockTab, p: DockProps): ReactNode {
  switch (tab) {
    case "memory":
      return <MemoryTab key={p.task.id} task={p.task} />;
    case "changes": {
      const home = p.projects?.find((x) => x.id === p.task.project_id);
      return <ChangesTab key={p.task.id} task={p.task} {...(home ? { homeName: repoName(home.root) } : {})} />;
    }
    case "overlap":
      return <OverlapTab key={p.task.id} task={p.task} projects={p.projects ?? []} />;
    case "ticket":
      return <TicketTab key={p.task.id} task={p.task} />;
    case "test":
      return <TestTab key={p.task.id} task={p.task} projects={p.projects ?? []} />;
    case "commit":
      return <CommitTab key={p.task.id} task={p.task} settings={p.settings} />;
    case "team":
      return p.teamSlot ?? <EmptyState size="sm" title="The team view arrives with the canvas" />;
  }
}

const NO_OVERLAPS = { count: 0, pending: 0 };

/**
 * "overlap" for the task once its pending decisions go from none to some (§7: the tab opens while a decision is
 * pending), until the developer picks a tab; the remembered tab per stage is left as it was.
 */
function useOverlapOpener(taskId: string, pending: number): [DockTab | null, () => void] {
  const [seen, setSeen] = useState({ taskId, pending: 0, forced: false });
  if (seen.taskId !== taskId || seen.pending !== pending) {
    const before = seen.taskId === taskId ? seen.pending : 0;
    const forced = before === 0 && pending > 0 ? true : seen.taskId === taskId && seen.forced;
    setSeen({ taskId, pending, forced });
  }
  const clear = () => setSeen((s) => ({ ...s, forced: false }));
  return [seen.forced ? "overlap" : null, clear];
}

export function Dock(p: DockProps): ReactElement | null {
  const { task, settings, open, store = dockStore } = p;
  const remembered = store.store.useSelector((s) => s.tabByStage[task.stage]);
  const height = store.store.useSelector((s) => s.heightByStage[task.stage]);
  // §6.3: the Changes tab counts the task's files as they change.
  const changed = changesStore.store.useSelector((s) =>
    (s.byTask[task.id]?.repos ?? []).reduce((n, r) => n + r.files.length, 0),
  );
  const overlaps = overlapStore.useSelector((s) => s.counts[task.id] ?? NO_OVERLAPS);
  const [forced, clearForced] = useOverlapOpener(task.id, overlaps.pending);
  useEffect(() => {
    if (!open) return;
    void changesStore.refresh(task.id);
    void refreshOverlaps(task.id);
  }, [open, task.id]);
  if (!open) return null;
  // §6.3: a stuck merge is settled in the Overlap tab even when no overlap rows remain.
  const tabs = visibleTabs(
    task.stage,
    settings.testCommand !== null,
    overlaps.count > 0 || mergeNeedsYou(task) !== null,
  );
  const tab = forced && tabs.includes(forced) ? forced : resolveTab(task.stage, remembered, tabs);
  const select = (next: string) => {
    clearForced();
    store.selectTab(task.stage, next as DockTab);
    if (next === "ticket") p.onOpenTicket?.();
  };
  return (
    <Panel
      as="aside"
      variant="plain"
      aria-label="Dock"
      data-position={settings.dockPosition === "Right" ? "right" : "bottom"}
      className="flex h-full min-h-0 flex-col bg-page"
    >
      <Tabs value={tab} onValueChange={select} className="flex h-full min-h-0 flex-col">
        <DockTabStrip
          tabs={tabs}
          grown={(height ?? 0) >= GROWN_HEIGHT}
          onToggleGrow={() => store.toggleGrow(task.stage)}
          counts={{ changes: changed, overlap: overlaps.count }}
          tones={{ overlap: overlaps.pending > 0 ? "warn" : "neutral" }}
        />
        <TabsContent value={tab} className="min-h-0 flex-1 overflow-auto">
          {body(tab, p)}
        </TabsContent>
      </Tabs>
    </Panel>
  );
}
