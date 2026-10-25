// One task's LiveSpec analysis loop, kept outside React so a timer survives a re-render or a hidden pane. Ported from
// LiveSpec's TabSession and bound to one task's spec.md and its LiveSpec level: it waits until you pause, then asks
// Plan 05's `livespec_analyze` (which reads the saved spec.md, the project's level and the previous analysed draft
// itself); if you keep writing, the last notes stay on screen marked out of date. Old responses are dropped by
// request id. LiveSpec's history, sections, context keys and activation went away with the tab model.
import { useCallback, useSyncExternalStore } from "react";
import type { LiveSpecEvent, LiveSpecLevel, TaskId } from "@/lib/bindings";
import { errorMessage } from "@/lib/ipcResult";
import { applyEvent, initialAnalysis, startAnalysis, type AnalysisState } from "@/livespec/analysisState";

export const MIN_CHARS = 24;
export const DEBOUNCE_MS = 1200;
export const normalize = (s: string) => s.replace(/\s+/g, " ").trim();

export interface SpecSessionDeps {
  /** The store saves the draft before calling livespec_analyze (Task 4). */
  analyze: (taskId: TaskId, onEvent: (e: LiveSpecEvent) => void) => Promise<void>;
  cancel: (taskId: TaskId) => Promise<void>;
  debounceMs?: () => number;
}

/** What React renders. A new object only when something changed (useSyncExternalStore). */
export interface SpecView {
  draft: string;
  analysis: AnalysisState;
  tooShort: boolean;
  stale: boolean;
  level: LiveSpecLevel;
}

export class SpecSession {
  readonly taskId: TaskId;
  private readonly deps: SpecSessionDeps;
  private listeners = new Set<() => void>();
  private state!: SpecView;
  private draft: string;
  private level: LiveSpecLevel;
  private analysis: AnalysisState = initialAnalysis;
  /** The normalised draft last sent: the same draft is not sent twice. */
  private lastSent = "";
  private timer: ReturnType<typeof setTimeout> | null = null;
  private requestId = 0;
  private disposed = false;

  constructor(taskId: TaskId, draft: string, level: LiveSpecLevel, deps: SpecSessionDeps) {
    this.taskId = taskId;
    this.draft = draft;
    this.level = level;
    this.deps = deps;
    this.emit();
  }

  getState(): SpecView {
    return this.state;
  }

  subscribe(fn: () => void): () => void {
    this.listeners.add(fn);
    return () => {
      this.listeners.delete(fn);
    };
  }

  setDraft(text: string): void {
    if (text === this.draft || this.disposed) return;
    this.draft = text;
    this.schedule();
    this.emit();
  }

  /** Off cancels any run and keeps the notes on screen; another level re-reads the same draft past the gate. */
  setLevel(level: LiveSpecLevel): void {
    if (level === this.level || this.disposed) return;
    this.level = level;
    if (level === "Off") {
      this.clearTimer();
      this.requestId++;
      if (this.analysis.status === "thinking") this.analysis = { ...this.analysis, status: "done" };
      void this.deps.cancel(this.taskId).catch(() => {});
    } else {
      this.lastSent = "";
      this.schedule();
    }
    this.emit();
  }

  /** "Try again": always re-analyses, past the gate. */
  rerun(): void {
    if (!this.disposed && this.level !== "Off") this.run();
  }

  /** Clears the timer and drops late events. */
  dispose(): void {
    this.disposed = true;
    this.clearTimer();
    this.requestId++;
    this.listeners.clear();
  }

  private schedule() {
    this.clearTimer();
    const norm = normalize(this.draft);
    if (this.level === "Off" || norm.length < MIN_CHARS || norm === this.lastSent) return;
    this.timer = setTimeout(() => {
      this.timer = null;
      this.run();
    }, this.deps.debounceMs?.() ?? DEBOUNCE_MS);
  }

  private clearTimer() {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  }

  private run() {
    this.clearTimer();
    const id = ++this.requestId;
    const text = this.draft;
    this.lastSent = normalize(text);
    this.analysis = startAnalysis(this.analysis, text);
    this.emit();
    const mine = () => id === this.requestId;
    const fail = (e: unknown) => {
      if (!mine()) return;
      this.analysis = { ...this.analysis, status: "error", error: errorMessage(e) };
      this.emit();
    };
    let pending: Promise<void>;
    try {
      pending = this.deps.analyze(this.taskId, (ev) => {
        if (!mine()) return;
        this.analysis = applyEvent(this.analysis, ev);
        this.emit();
      });
    } catch (e) {
      fail(e); // a synchronous throw ends like a rejection, not a session stuck on "thinking"
      return;
    }
    pending.catch(fail);
  }

  private emit() {
    const norm = normalize(this.draft);
    this.state = {
      draft: this.draft,
      analysis: this.analysis,
      tooShort: norm.length < MIN_CHARS,
      stale: this.analysis.status !== "idle" && normalize(this.analysis.analyzedDraft) !== norm,
      level: this.level,
    };
    this.listeners.forEach((l) => l());
  }
}

/** The session's state, re-rendering on each change; null without a session. */
export function useSpecSession(s: SpecSession | null): SpecView | null {
  const subscribe = useCallback((onChange: () => void) => (s ? s.subscribe(onChange) : () => {}), [s]);
  const read = useCallback(() => (s ? s.getState() : null), [s]);
  return useSyncExternalStore(subscribe, read, read);
}
