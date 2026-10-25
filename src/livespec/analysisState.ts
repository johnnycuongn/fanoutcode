// Pure state transitions for the live analysis, testable without React. Copied from LiveSpec
// (src/lib/analysisState.ts) and moved onto Plan 05's `LiveSpecEvent` (A14). Check events may arrive before or after
// `Done`: the fact-check starts when the Conflicts lane finishes, independently of the other lanes. The previous-draft
// bookkeeping LiveSpec kept here moved to the backend, which remembers the last analysed draft per task.
import type { LiveSpecCheck, LiveSpecEvent } from "@/lib/bindings";
import { appendLane, joinLanes } from "./lanes";

export type AnalysisStatus = "idle" | "thinking" | "done" | "error";

export interface AnalysisState {
  status: AnalysisStatus;
  lanes: string[];
  markdown: string;
  error: string | null;
  analyzedDraft: string;
  checking: { count: number; total: number } | null;
  checks: LiveSpecCheck[];
  /** Every non-fatal message for the notes on screen, oldest first. */
  notices: string[];
  /** The fact-check ended early (an edit skipped past it). */
  checksStopped: boolean;
  /** Some sections couldn't be written; offer a retry. */
  partial: boolean;
}

export const initialAnalysis: AnalysisState = {
  status: "idle",
  lanes: [],
  markdown: "",
  error: null,
  analyzedDraft: "",
  checking: null,
  checks: [],
  notices: [],
  checksStopped: false,
  partial: false,
};

/** A new pause: the old notes stay visible until the backend says `Started`. */
export const startAnalysis = (prev: AnalysisState, draft: string): AnalysisState => ({
  ...prev,
  status: "thinking",
  error: null,
  analyzedDraft: draft,
});

export function applyEvent(s: AnalysisState, ev: LiveSpecEvent): AnalysisState {
  switch (ev.type) {
    case "Text": {
      const lanes = appendLane(s.lanes, ev.lane, ev.text);
      return { ...s, lanes, markdown: joinLanes(lanes) };
    }
    case "Done":
      return { ...s, status: "done", markdown: ev.text || s.markdown, partial: ev.partial };
    case "Error":
      return { ...s, status: "error", error: ev.message };
    case "Checking":
      return { ...s, checking: { count: ev.count, total: ev.total } };
    case "Check":
      return { ...s, checks: [...s.checks, ev.result] };
    case "Notice":
      return { ...s, notices: [...s.notices, ev.message] };
    case "Started":
      return {
        ...s,
        lanes: [],
        markdown: "",
        checks: [],
        checking: null,
        notices: [],
        checksStopped: false,
        partial: false,
        error: null,
      };
    case "Skipped": {
      // The skip aborted any fact-checks still running for these notes.
      const unfinished = s.checking !== null && s.checks.length < s.checking.count;
      return unfinished
        ? { ...s, status: "done", notices: [...s.notices, "Checks stopped when you edited."], checksStopped: true }
        : { ...s, status: "done" };
    }
    default:
      return s;
  }
}
