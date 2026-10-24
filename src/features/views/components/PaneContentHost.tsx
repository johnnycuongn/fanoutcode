// What a pane shows: the picker prompt, a task page (Plan 08), a terminal (Plan 04) or a team canvas (Plan 09).
// Those three load lazily so this feature builds before their plans land; a task that is gone or archived shows a
// placeholder instead of throwing, so the other panes still restore (Review Focus 1).
import { lazy, Suspense, useCallback, useEffect, useState, type ReactElement, type ReactNode } from "react";
import { Button, EmptyState } from "@/components/ui";
import type { PaneContent, PaneId, Task, TaskId, ViewId } from "@/lib/bindings";
import { projectsStore, selectProjectList } from "@/lib/stores/projectsStore";
import { tasksStore } from "@/lib/stores/tasksStore";
import { setActiveTab, shellStore } from "@/features/shell";
import type { TaskPageHandle } from "@/features/task-page";
import { setPaneHandle } from "../paneHandles";
import { openPicker, openTeamPane } from "../store";
import type { TerminalPaneHandle } from "./TerminalPane";

const TaskPage = lazy(() => import("@/features/task-page").then((m) => ({ default: m.TaskPage })));
const CanvasPane = lazy(() => import("@/features/canvas").then((m) => ({ default: m.CanvasPane })));
const TerminalPane = lazy(() => import("./TerminalPane").then((m) => ({ default: m.TerminalPane })));

type RenderTeam = (task: Task) => ReactNode;
let renderTeamLoad: Promise<RenderTeam | undefined> | null = null;
/** Plan 09's `renderTeam`, once its module exports one; undefined until then. */
function useRenderTeam(): RenderTeam | undefined {
  const [fn, setFn] = useState<{ current?: RenderTeam }>({});
  useEffect(() => {
    let live = true;
    renderTeamLoad ??= import("@/features/canvas")
      .then((m) => (m as { renderTeam?: RenderTeam }).renderTeam)
      .catch(() => undefined);
    void renderTeamLoad.then((r) => live && r && setFn({ current: r }));
    return () => {
      live = false;
    };
  }, []);
  return fn.current;
}

export type PaneContentHostProps = { viewId: ViewId; pane: PaneId; content: PaneContent; focused: boolean };

function Placeholder({ title, action, onPick }: { title: string; action: string; onPick(): void }): ReactElement {
  return (
    <EmptyState
      className="h-full justify-center"
      title={title}
      action={
        <Button variant="secondary" size="sm" onClick={onPick}>
          {action}
        </Button>
      }
    />
  );
}

function TaskBody({ viewId, pane, taskId }: { viewId: ViewId; pane: PaneId; taskId: TaskId }): ReactElement | null {
  const container = shellStore.useSelector((s) => s.container);
  const projects = projectsStore.useSelector(selectProjectList);
  const renderTeam = useRenderTeam();
  const ref = useCallback(
    (h: TaskPageHandle | null) => {
      setPaneHandle(pane, h ? { kind: "task", handle: h } : null);
      return () => setPaneHandle(pane, null);
    },
    [pane],
  );
  if (!container) return null;
  return (
    <Suspense fallback={null}>
      <TaskPage
        ref={ref}
        taskId={taskId}
        paneId={pane}
        container={container}
        projects={projects}
        {...(renderTeam ? { renderTeam } : {})}
        onBack={() => setActiveTab("board")}
        onOpenTeamPane={() => openTeamPane(viewId, pane, taskId)}
      />
    </Suspense>
  );
}

function TerminalBody({ pane, sessionId, focused }: { pane: PaneId; sessionId: string; focused: boolean }) {
  const ref = useCallback(
    (h: TerminalPaneHandle | null) => {
      setPaneHandle(pane, h ? { kind: "terminal", handle: h } : null);
      return () => setPaneHandle(pane, null);
    },
    [pane],
  );
  return (
    <Suspense fallback={null}>
      <TerminalPane ref={ref} sessionId={sessionId} paneId={pane} focused={focused} />
    </Suspense>
  );
}

export function PaneContentHost({ viewId, pane, content, focused }: PaneContentHostProps): ReactElement | null {
  const taskId = content.type === "Task" || content.type === "TeamCanvas" ? content.task_id : null;
  const available = tasksStore.useSelector((s) => {
    const t = taskId ? s.byId[taskId] : undefined;
    return Boolean(t && !t.archived);
  });
  const pick = () => openPicker(viewId, pane);
  switch (content.type) {
    case "Empty":
      return <Placeholder title="Empty pane" action="Pick a task" onPick={pick} />;
    case "Task":
      return available ? (
        <TaskBody viewId={viewId} pane={pane} taskId={content.task_id} />
      ) : (
        <Placeholder title="Task no longer available" action="Pick another task" onPick={pick} />
      );
    case "TeamCanvas":
      return available ? (
        <Suspense fallback={null}>
          <CanvasPane taskId={content.task_id} />
        </Suspense>
      ) : (
        <Placeholder title="Task no longer available" action="Pick another task" onPick={pick} />
      );
    case "Terminal":
      return <TerminalBody pane={pane} sessionId={content.session_id} focused={focused} />;
  }
}
