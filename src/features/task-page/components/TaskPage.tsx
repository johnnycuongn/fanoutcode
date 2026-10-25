// The task page in a pane (§6, §10.3): header, body and dock, and the TaskPageHandle the shell routes ⌘⏎, ⌘E, ⌘⇧A,
// ⌘J, ⌘⇧J and ⌃` to (A5). The Task record is read from Plan 06's tasksStore through taskSource; whether this pane's
// dock is open and what its body shows are this pane's own state.
import {
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactElement,
  type ReactNode,
  type Ref,
} from "react";
import { EmptyState, toast } from "@/components/ui";
import type { Container, PaneId, Project, Task, TaskId } from "@/lib/bindings";
import { Dock, DockSplit, dockStore, nextOnGrowShortcut, type BodyMode, type DockTab } from "@/features/dock";
import { consumeDockTab } from "@/features/shell";
import { SAVE_FAILED, specStore } from "@/features/spec-editor";
import type { TerminalPaneHandle } from "@/features/views";
import { wantsSession } from "../body";
import { acquireTaskPageEvents } from "../events";
import { linksChanged } from "../links";
import { primaryActionFor } from "../primaryAction";
import { DEFAULT_TASK_SETTINGS, taskPageStore } from "../store";
import { tasksSource } from "../taskSource";
import { toggleWorktree } from "../worktree";
import { TaskBody } from "./TaskBody";
import { TaskHeader } from "./TaskHeader";
import { useBringBaseIn } from "./useBringBaseIn";

export interface TaskPageHandle {
  primaryAction(): void;
  rename(): void;
  archive(): void;
  toggleDock(): void;
  growTeam(): void;
  toggleTerminalStrip(): void;
  focusTerminal(): void;
}

export interface TaskPageProps {
  taskId: TaskId;
  paneId: PaneId;
  container: Container;
  projects: Project[];
  renderTeam?: (task: Task) => ReactNode;
  onBack(): void;
  onOpenTeamPane?(task: Task): void;
  ref?: Ref<TaskPageHandle>;
}

export function TaskPage({
  taskId,
  paneId,
  container,
  projects,
  renderTeam,
  onBack,
  onOpenTeamPane,
  ref,
}: TaskPageProps): ReactElement {
  const task = useSyncExternalStore(tasksSource.subscribe, () => tasksSource.get(taskId));
  const settings = taskPageStore.store.useSelector((s) => s.settings[taskId]) ?? DEFAULT_TASK_SETTINGS;
  const queued = taskPageStore.store.useSelector((s) => taskId in s.queued);
  const cost = taskPageStore.store.useSelector((s) => s.cost[taskId]);
  const contextPct = taskPageStore.store.useSelector((s) => s.contextPct[taskId]);
  const [dockOpen, setDockOpen] = useState<boolean | null>(null);
  const [body, setBody] = useState<BodyMode>("terminal");
  const [renaming, setRenaming] = useState(false);
  const terminalRef = useRef<TerminalPaneHandle | null>(null);
  const bringBaseIn = useBringBaseIn(task);

  useEffect(() => acquireTaskPageEvents(), []);
  useEffect(() => {
    void taskPageStore.open(taskId).then((t) => {
      const tab = consumeDockTab(taskId);
      if (tab) {
        dockStore.selectTab(t.stage, tab);
        setDockOpen(true);
      }
    });
  }, [taskId]);
  // Entering a stage resets the dock as the stage wants it (Build grows on Team) and brings the terminal back.
  useEffect(() => {
    let last = tasksSource.get(taskId)?.stage;
    return tasksSource.subscribe(() => {
      const t = tasksSource.get(taskId);
      if (!t || t.stage === last) return;
      last = t.stage;
      const e = dockStore.enterStage(t.stage);
      if (e.open !== null) setDockOpen(e.open);
      setBody("terminal");
    });
  }, [taskId]);
  // Plan 04: the page opens (or resumes) the task's session; sessions_open is idempotent.
  const needsSession = task ? wantsSession(task) : false;
  useEffect(() => {
    if (needsSession) void taskPageStore.startSession(taskId);
  }, [taskId, needsSession]);

  const open = dockOpen ?? (task ? dockStore.defaultOpen(task.stage) : false);
  const primary = task ? primaryActionFor(task, { mayCommit: settings.mayCommit, queued }) : null;
  const focusTerminal = () => terminalRef.current?.focus();
  const openDockTab = (tab: DockTab) => {
    if (!task) return;
    dockStore.selectTab(task.stage, tab);
    setDockOpen(true);
  };
  const runPrimary = async () => {
    if (!task) return;
    if (task.stage === "Spec" && !task.archived) {
      try {
        if (!(await specStore.approve(task))) toast.error(SAVE_FAILED);
      } catch (e) {
        toast.error(typeof e === "string" ? e : String(e));
      }
    } else await taskPageStore.runPrimary(taskId, focusTerminal, openDockTab);
  };
  const growTeam = () => {
    if (!task) return;
    const placement =
      settings.teamViewPlacement === "OwnPane" && !onOpenTeamPane ? "DockGrows" : settings.teamViewPlacement;
    const grown = dockStore.isGrown(task.stage);
    const next = nextOnGrowShortcut(placement, { open, grown, body });
    if (next.openTeamPane) return onOpenTeamPane?.(task);
    setDockOpen(next.open);
    setBody(next.body);
    if (next.grown !== grown) dockStore.toggleGrow(task.stage);
    if (next.tab) dockStore.selectTab(task.stage, next.tab);
  };

  useImperativeHandle(ref, () => ({
    primaryAction: () => void runPrimary(),
    rename: () => setRenaming(true),
    archive: () => void taskPageStore.act(taskId, { type: task?.archived ? "Restore" : "Archive" }),
    toggleDock: () => setDockOpen(!open),
    growTeam,
    toggleTerminalStrip: () => {
      if (task?.stage === "Spec") specStore.toggleTerminal(taskId);
    },
    focusTerminal,
  }));

  if (!task || !primary) return <EmptyState className="h-full justify-center" title="Opening task…" />;
  const project = projects.find((x) => x.id === task.project_id) ?? null;
  const team = renderTeam?.(task) ?? null;
  return (
    <div className="flex h-full min-h-0 flex-col bg-page">
      <TaskHeader
        task={task}
        project={project}
        showProjectBadge={container.type === "Workspace"}
        settings={settings}
        cost={cost}
        contextPct={contextPct}
        primary={primary}
        linksChanged={linksChanged(task, projects)}
        renaming={renaming}
        onRenamingChange={setRenaming}
        onPrimary={() => void runPrimary()}
        onBack={onBack}
        onRename={(title) => void taskPageStore.rename(taskId, title)}
        onAction={(a) => void taskPageStore.act(taskId, a)}
        onStopAgent={() => void taskPageStore.stopAgent(taskId)}
        onLinkIssue={() => openDockTab("ticket")}
        onIntegrate={bringBaseIn.start}
        onToggleWorktree={() => toggleWorktree(task)}
      />
      <div className="min-h-0 flex-1">
        <DockSplit
          stage={task.stage}
          position={settings.dockPosition}
          open={open}
          body={
            <TaskBody
              task={task}
              paneId={paneId}
              mode={body}
              team={team}
              terminalRef={terminalRef}
              onStartSession={() => void taskPageStore.startSession(taskId)}
              onShowTerminal={() => setBody("terminal")}
            />
          }
          dock={<Dock task={task} settings={settings} open={open} teamSlot={team} projects={projects} />}
        />
      </div>
      {bringBaseIn.dialog}
    </div>
  );
}
