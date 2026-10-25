import { beforeEach, expect, test, vi } from "vitest";
import type { LiveSpecEvent, LiveSpecLevel } from "@/lib/bindings";
import { SpecSession, type SpecSessionDeps } from "./specSession";

const LONG = "A spec long enough to be analysed by the notes model.";
function harness(level: LiveSpecLevel) {
  const calls: { level: LiveSpecLevel; draft: string }[] = []; // the session's view at call time; the command itself takes only the task id
  const emits: ((e: LiveSpecEvent) => void)[] = [];
  // eslint-disable-next-line prefer-const -- assigned after deps, which close over it
  let s: SpecSession;
  const deps: SpecSessionDeps = {
    analyze: vi.fn(async (_id, onEvent) => {
      const v = s.getState();
      calls.push({ level: v.level, draft: v.draft });
      emits.push(onEvent);
    }),
    cancel: vi.fn(async () => {}),
  };
  s = new SpecSession("task-1", "", level, deps);
  return {
    calls,
    deps,
    emit: (e: LiveSpecEvent) => emits.at(-1)!(e),
    emitFrom: (i: number, e: LiveSpecEvent) => emits[i](e),
    s,
  };
}
beforeEach(() => vi.useFakeTimers());

test("Off never calls analyze", () => {
  const h = harness("Off");
  h.s.setDraft(LONG);
  vi.advanceTimersByTime(5000);
  expect(h.calls).toHaveLength(0);
});
test("Simple runs once after a 1200 ms pause with at least 24 characters", () => {
  const h = harness("Simple");
  h.s.setDraft("too short");
  vi.advanceTimersByTime(2000);
  expect(h.calls).toHaveLength(0);
  h.s.setDraft(LONG);
  vi.advanceTimersByTime(1199);
  expect(h.calls).toHaveLength(0);
  vi.advanceTimersByTime(1);
  expect(h.calls).toEqual([{ level: "Simple", draft: LONG }]);
  expect(h.deps.analyze).toHaveBeenCalledWith("task-1", expect.any(Function));
  expect(h.s.getState().analysis.status).toBe("thinking");
});
test("the same normalized draft is not sent twice; rerun always sends", () => {
  const h = harness("Simple");
  h.s.setDraft(LONG);
  vi.advanceTimersByTime(1200);
  h.s.setDraft(LONG + "  ");
  vi.advanceTimersByTime(1200);
  expect(h.calls).toHaveLength(1);
  h.s.rerun();
  expect(h.calls).toHaveLength(2);
});
test("done records the draft as analysed and the next edit marks the notes stale", () => {
  const h = harness("Simple");
  h.s.setDraft(LONG);
  vi.advanceTimersByTime(1200);
  h.emit({ type: "Started" });
  h.emit({ type: "Text", text: "## Gaps\n- a", lane: 0 });
  h.emit({ type: "Done", text: "## Gaps\n- a", partial: false });
  expect(h.s.getState().analysis.status).toBe("done");
  expect(h.s.getState().stale).toBe(false);
  h.s.setDraft(LONG + " more");
  expect(h.s.getState().stale).toBe(true);
  vi.advanceTimersByTime(1200);
  expect(h.calls).toHaveLength(2);
  expect(h.calls[1].draft).toBe(LONG + " more");
});
test("switching Simple to Heavy re-reads the same draft past the gate", () => {
  const h = harness("Simple");
  h.s.setDraft(LONG);
  vi.advanceTimersByTime(1200);
  h.s.setLevel("Heavy");
  vi.advanceTimersByTime(1200);
  expect(h.calls).toEqual([
    { level: "Simple", draft: LONG },
    { level: "Heavy", draft: LONG },
  ]);
});
test("switching to Off cancels the run and keeps the notes on screen", () => {
  const h = harness("Simple");
  h.s.setDraft(LONG);
  vi.advanceTimersByTime(1200);
  h.emit({ type: "Started" });
  h.emit({ type: "Done", text: "## Gaps\n- a", partial: false });
  h.s.setLevel("Off");
  expect(h.deps.cancel).toHaveBeenCalledWith("task-1");
  expect(h.s.getState().analysis.markdown).toBe("## Gaps\n- a");
  h.s.setDraft(LONG + " x");
  vi.advanceTimersByTime(5000);
  expect(h.calls).toHaveLength(1);
});
test("events from a superseded run are dropped", () => {
  const h = harness("Simple");
  h.s.setDraft(LONG);
  vi.advanceTimersByTime(1200);
  h.s.rerun();
  h.emitFrom(0, { type: "Text", text: "old", lane: 0 });
  h.emit({ type: "Started" });
  h.emit({ type: "Text", text: "new", lane: 0 });
  expect(h.s.getState().analysis.markdown).toBe("new");
});
test("a rejected analyze call ends in the error state with its message", async () => {
  const deps: SpecSessionDeps = {
    analyze: vi.fn(async () => {
      throw "Install Claude Code first.";
    }),
    cancel: vi.fn(async () => {}),
  };
  const s = new SpecSession("task-1", "", "Simple", deps);
  s.setDraft(LONG);
  vi.advanceTimersByTime(1200);
  await Promise.resolve();
  expect(s.getState().analysis).toMatchObject({ status: "error", error: "Install Claude Code first." });
});
