// The Overlap tab's right column (§5.4, §7): for the selected file, the other task's changes beside what this task
// will write, and, while a decision is pending, the four answers with Continue; a settled overlap shows both diffs
// and the decision history instead.
import { useState, type ReactElement } from "react";
import { Button, RadioGroup, statusLabel, type RadioOption } from "@/components/ui";
import type { Decision, OverlapRow, PendingDecision, Task } from "@/lib/bindings";
import { agentStatus } from "@/lib/format";
import { tasksStore } from "@/lib/stores/tasksStore";
import { requestDockTab } from "@/features/shell";
import { decide } from "../store";
import { commandEffect, otherTitle, relationLabel } from "../labels";
import { DiffBox } from "./DiffBox";
import { WillWriteBox } from "./WillWriteBox";

export interface DecisionPaneProps {
  task: Task;
  row: OverlapRow;
}

type Choice = Decision["type"];

/** §5.4's four answers; a shell command can only go ahead (granted for this one command) or wait. */
function choices(other: string, task: string, pending: PendingDecision | null): RadioOption<Choice>[] {
  if (pending?.type === "Command")
    return [
      {
        value: "OnTopAll",
        label: "Proceed",
        description: commandEffect(pending.command, pending.kind, other, pending.paths),
      },
      {
        value: "Hold",
        label: "Hold",
        description: `Don't run it; you are reminded when ${other} commits these files.`,
      },
    ];
  return [
    {
      value: "OnTopFile",
      label: `Proceed on top of ${other}'s changes, check the next file`,
      description: `${task} keeps ${other}'s changes and adds its own around them; the next file in use is asked about.`,
    },
    {
      value: "OnTopAll",
      label: `Proceed on top for every file ${other} touches`,
      description: `No further files ${other} touches are asked about this session.`,
    },
    {
      value: "Replace",
      label: `Replace ${other}'s changes in this file`,
      description: `${other}'s uncommitted changes here may go; the file is marked taken over in both tasks.`,
      tone: "bad",
    },
    { value: "Hold", label: "Hold", description: `Nothing is written; ${task} waits until ${other} commits the file.` },
  ];
}

function when(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
}

function Choices({ task, row, other }: DecisionPaneProps & { other: string }): ReactElement {
  const options = choices(other, task.title, row.pending);
  const [choice, setChoice] = useState<Choice>(options[0].value);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const go = async () => {
    setBusy(true);
    setError(null);
    try {
      await decide(task.id, row, { type: choice });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="grid gap-2">
      <RadioGroup<Choice>
        aria-label={`What ${task.title} does with ${row.path}`}
        options={options}
        value={choice}
        onValueChange={setChoice}
      />
      {choice !== "Hold" && row.other_task && <p className="text-xs text-mute">{`${other}'s session is told.`}</p>}
      {error && (
        <p role="alert" className="text-xs text-bad">
          {error}
        </p>
      )}
      <div className="flex justify-end gap-2">
        <Button variant="secondary" size="sm" onClick={() => requestDockTab(task.id, "changes")}>
          Open in Changes
        </Button>
        <Button variant="primary" size="sm" loading={busy} onClick={() => void go()}>
          Continue
        </Button>
      </div>
    </div>
  );
}

function History({ task, row }: DecisionPaneProps): ReactElement {
  const label = relationLabel(row);
  return (
    <div className="grid gap-2">
      <p className="text-xs text-ink">
        {row.decided_at ? `Decided ${when(row.decided_at)}: ${label}` : `No decision was needed: ${label}`}
      </p>
      <div className="flex justify-end">
        <Button variant="secondary" size="sm" onClick={() => requestDockTab(task.id, "changes")}>
          Open in Changes
        </Button>
      </div>
    </div>
  );
}

export function DecisionPane({ task, row }: DecisionPaneProps): ReactElement {
  const byId = tasksStore.useSelector((s) => s.byId);
  const otherTask = row.other_task ? byId[row.other_task] : undefined;
  const other = otherTitle(row.other_task, byId);
  // With no task behind the claim, the shared tree's diff against this task's base is the closest view of it.
  const otherId = row.other_task ?? task.id;
  const meta = [
    otherTask?.stage,
    otherTask ? statusLabel(agentStatus(otherTask.state)) : null,
    `last edit ${when(row.updated_at)}`,
    relationLabel(row),
  ].filter(Boolean);
  return (
    <section aria-label={row.path} className="grid min-w-0 content-start gap-3 p-3">
      <header className="grid gap-0.5">
        <h3 className="text-sm font-semibold text-ink">{`${other} is also changing ${row.path}`}</h3>
        <p className="text-xs text-mute">{meta.join(" · ")}</p>
      </header>
      <div className="grid min-w-0 grid-cols-2 gap-2">
        <DiffBox who={other} taskId={otherId} repo={row.repo_root} path={row.path} />
        {row.pending ? (
          <WillWriteBox taskTitle={task.title} taskId={task.id} repo={row.repo_root} path={row.path} />
        ) : (
          <DiffBox who={task.title} taskId={task.id} repo={row.repo_root} path={row.path} />
        )}
      </div>
      {row.pending ? <Choices task={task} row={row} other={other} /> : <History task={task} row={row} />}
    </section>
  );
}
