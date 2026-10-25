// Replay of a finished Build (§10.7 "Data"): a slider over the task's events (by position in the log, so gaps in
// `seq` never land on a missing event), Play/Pause at one event per 400 ms, and Live to return to the present.
import { useEffect, useState, type ReactElement } from "react";
import { Button, Slider } from "@/components/ui";
import { clockTime } from "../format";
import { setReplaySeq, stopReplay, useCanvas } from "../store";

export const REPLAY_STEP_MS = 400;

export function ReplayScrubber({ taskId }: { taskId: string }): ReactElement | null {
  const { events, replaySeq } = useCanvas(taskId);
  const [playing, setPlaying] = useState(false);
  const index = replaySeq === null ? -1 : events.findIndex((e) => e.seq >= replaySeq);
  const at = index < 0 ? events.length - 1 : index;
  const last = events.length - 1;

  useEffect(() => {
    if (!playing || replaySeq === null) return;
    const t = setTimeout(() => {
      if (at >= last) setPlaying(false);
      else setReplaySeq(taskId, events[at + 1].seq);
    }, REPLAY_STEP_MS);
    return () => clearTimeout(t);
  }, [playing, replaySeq, at, last, events, taskId]);

  if (replaySeq === null || events.length === 0) return null;
  return (
    <div data-replay="" className="flex items-center gap-2 border-t border-line bg-bar px-3 py-1.5">
      <Button size="sm" variant="secondary" onClick={() => setPlaying(!playing)}>
        {playing ? "Pause" : "Play"}
      </Button>
      <Slider
        className="min-w-0 flex-1"
        value={at}
        min={0}
        max={Math.max(0, last)}
        onChange={(i) => setReplaySeq(taskId, events[i].seq)}
        label="Replay position"
        formatValue={(i) => clockTime(events[i].at)}
      />
      <span aria-live="polite" className="shrink-0 text-xs text-mute tabular-nums">
        {`${at + 1} of ${events.length} events`}
      </span>
      <Button
        size="sm"
        variant="ghost"
        onClick={() => {
          setPlaying(false);
          stopReplay(taskId);
        }}
      >
        Live
      </Button>
    </div>
  );
}
