// The task page's body: the Spec stage editor, the task's terminal, a start-session prompt, or (Swap placement)
// the team in the body with the terminal as a strip (§10.7).
import type { ReactElement, ReactNode, Ref } from "react";
import { Button, EmptyState } from "@/components/ui";
import type { PaneId, Task } from "@/lib/bindings";
import type { BodyMode } from "@/features/dock";
import { SpecStagePage, TerminalStrip } from "@/features/spec-editor";
import { TerminalPane, type TerminalPaneHandle } from "@/features/views";
import { bodyFor } from "../body";

export interface TaskBodyProps {
  task: Task;
  paneId: PaneId;
  mode: BodyMode;
  team: ReactNode;
  terminalRef: Ref<TerminalPaneHandle>;
  onStartSession(): void;
  onShowTerminal(): void;
}

export function TaskBody({
  task,
  paneId,
  mode,
  team,
  terminalRef,
  onStartSession,
  onShowTerminal,
}: TaskBodyProps): ReactElement {
  const session = task.primary_session;
  // A new primary session (Start fresh session) remounts the terminal on the new id.
  const terminal = session ? (
    <TerminalPane key={session} ref={terminalRef} sessionId={session} paneId={paneId} />
  ) : null;
  const kind = bodyFor(task);
  if (kind === "spec") return <SpecStagePage task={task} terminal={terminal} />;
  if (mode === "team")
    return (
      <div className="flex h-full min-h-0 flex-col">
        <div className="min-h-0 flex-1">
          {team ?? <EmptyState size="sm" title="The team view arrives with the canvas" />}
        </div>
        <TerminalStrip
          expanded={false}
          hasSession={session !== null}
          onToggle={onShowTerminal}
          onStart={onStartSession}
        />
      </div>
    );
  if (kind === "terminal") return <div className="h-full min-h-0">{terminal}</div>;
  return (
    <EmptyState
      className="h-full justify-center"
      title="Starting session…"
      action={
        <Button size="sm" variant="secondary" onClick={onStartSession}>
          Start session
        </Button>
      }
    />
  );
}
