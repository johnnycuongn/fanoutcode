// The Spec stage body (§6.1): the spec in Literata on the left, the LiveSpec notes on the right, the level switch
// above, and the task's terminal as a strip below that expands into a resizable split.
import { useEffect, type ReactElement, type ReactNode } from "react";
import { Resizable } from "@/components/ui";
import type { Task } from "@/lib/bindings";
import { DraftPane, wordCount } from "@/livespec/DraftPane";
import { initialAnalysis } from "@/livespec/analysisState";
import { NotesPane } from "@/livespec/NotesPane";
import { useSpecSession } from "../specSession";
import { DEFAULT_TERMINAL_RATIO, specStore, type SpecStore } from "../store";
import { LiveSpecSwitch } from "./LiveSpecSwitch";
import { TerminalStrip } from "./TerminalStrip";

const PLACEHOLDER = "Write the spec here. Turn on LiveSpec to get notes as you pause.";

export interface SpecStagePageProps {
  task: Task;
  /** The task's TerminalPane (A5), or null when it has no session. */
  terminal: ReactNode | null;
  store?: SpecStore;
}

export function SpecStagePage({ task, terminal, store = specStore }: SpecStagePageProps): ReactElement {
  const st = store.store.useSelector((s) => s.byTask[task.id]);
  const level = store.store.useSelector((s) => s.levelByProject[task.project_id] ?? "Off");
  const view = useSpecSession(store.session(task.id));
  useEffect(() => {
    void store.open(task);
  }, [store, task]);

  const text = st?.text ?? "";
  const expanded = st?.terminalExpanded ?? false;
  const editor = (
    <div className="grid h-full min-h-0 grid-cols-[minmax(0,3fr)_minmax(320px,2fr)]">
      <DraftPane
        value={text}
        onChange={(v) => store.setText(task.id, v)}
        placeholder={PLACEHOLDER}
        readOnly={!st?.loaded}
      />
      <NotesPane
        analysis={view?.analysis ?? initialAnalysis}
        stale={view?.stale ?? false}
        level={level}
        onRerun={() => store.session(task.id)?.rerun()}
      />
    </div>
  );
  const strip = (
    <TerminalStrip
      expanded={expanded}
      hasSession={task.primary_session !== null}
      onToggle={() => store.toggleTerminal(task.id)}
      onStart={() => void store.startSession(task.id).catch(() => {})}
    />
  );

  return (
    <div data-scope="spec-page" className="flex h-full min-h-0 flex-col bg-page">
      <div className="flex h-9 shrink-0 items-center gap-3 border-b border-line bg-bar px-3 text-xs text-mute">
        <span className="tabular-nums">{wordCount(text)} words</span>
        {st?.saving === "failed" && <span className="text-bad">Couldn&apos;t save the spec</span>}
        <span className="flex-1" />
        <LiveSpecSwitch level={level} disabled={!st?.loaded} onChange={(l) => void store.setLevel(task, l)} />
      </div>
      {st?.error && (
        <p role="alert" className="shrink-0 border-b border-line bg-well px-3 py-2 text-[13px] text-bad">
          {st.error}
        </p>
      )}
      {expanded ? (
        <Resizable
          className="min-h-0 flex-1"
          direction="vertical"
          ratio={1 - (st?.terminalRatio ?? DEFAULT_TERMINAL_RATIO)}
          onRatioChange={(r) => store.setTerminalRatio(task.id, 1 - r)}
          handleLabel="Resize terminal"
          first={editor}
          second={
            <div className="flex h-full min-h-0 flex-col">
              {strip}
              <div className="min-h-0 flex-1">{terminal}</div>
            </div>
          }
        />
      ) : (
        <>
          <div className="min-h-0 flex-1">{editor}</div>
          {strip}
        </>
      )}
    </div>
  );
}
