// The dock (⌘J): a bottom panel or a right inspector holding the stage's tabs (§6, §10.3). The tab and height are
// remembered per stage in the dock store; whether it is open is the pane's own state.
import { useEffect, type ReactElement, type ReactNode } from "react";
import { EmptyState, Panel, Tabs, TabsContent } from "@/components/ui";
import type { Project, Task } from "@/lib/bindings";
import { ChangesTab, changesStore, repoName } from "@/features/changes";
import type { TaskSettings } from "@/features/task-page";
import { dockStore, type DockStore } from "../store";
import { GROWN_HEIGHT, resolveTab, visibleTabs } from "../tabRules";
import type { DockTab } from "../types";
import { DockTabStrip } from "./DockTabStrip";
import { MemoryTab } from "./MemoryTab";

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
    case "team":
      return p.teamSlot ?? <EmptyState size="sm" title="The team view arrives with the canvas" />;
    default:
      return <EmptyState size="sm" title="Nothing here yet" />;
  }
}

export function Dock(p: DockProps): ReactElement | null {
  const { task, settings, open, store = dockStore } = p;
  const remembered = store.store.useSelector((s) => s.tabByStage[task.stage]);
  const height = store.store.useSelector((s) => s.heightByStage[task.stage]);
  // §6.3: the Changes tab counts the task's files as they change.
  const changed = changesStore.store.useSelector((s) =>
    (s.byTask[task.id]?.repos ?? []).reduce((n, r) => n + r.files.length, 0),
  );
  useEffect(() => {
    if (open) void changesStore.refresh(task.id);
  }, [open, task.id]);
  if (!open) return null;
  const tabs = visibleTabs(task.stage, settings.testCommand !== null);
  const tab = resolveTab(task.stage, remembered, tabs);
  const select = (next: string) => {
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
          counts={{ changes: changed }}
        />
        <TabsContent value={tab} className="min-h-0 flex-1 overflow-auto">
          {body(tab, p)}
        </TabsContent>
      </Tabs>
    </Panel>
  );
}
