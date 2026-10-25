import { expect, test } from "vitest";
import { createDockStore } from "./store";
import { mem } from "./testing/fakes";

test("selectTab remembers per stage and persists", () => {
  const p = mem();
  const d = createDockStore(p);
  d.selectTab("Review", "memory");
  expect(d.tabFor("Review", false)).toBe("memory");
  expect(d.tabFor("Build", false)).toBe("team");
  expect(p.last?.tabByStage.Review).toBe("memory");
});
test("enterStage returns the entry state, grows Build and resets the stage's tab to its default", () => {
  const d = createDockStore(mem());
  d.selectTab("Build", "memory");
  expect(d.enterStage("Build")).toEqual({ open: true, grown: true, tab: "team" });
  expect(d.heightFor("Build")).toBe(0.67);
  expect(d.isGrown("Build")).toBe(true);
  expect(d.enterStage("Spec")).toEqual({ open: false, grown: false, tab: "memory" });
  expect(d.enterStage("Done")).toEqual({ open: null, grown: null, tab: "memory" });
  expect(d.heightFor("Review")).toBe(0.34);
});
test("defaultOpen is false only for Spec", () => {
  const d = createDockStore(mem());
  expect(d.defaultOpen("Spec")).toBe(false);
  expect(d.defaultOpen("Plan")).toBe(true);
});
test("heightFor defaults to 0.34 and toggleGrow flips to 0.67 and back, per stage", () => {
  const d = createDockStore(mem());
  expect(d.heightFor("Plan")).toBe(0.34);
  d.toggleGrow("Plan");
  expect(d.heightFor("Plan")).toBe(0.67);
  expect(d.heightFor("Build")).toBe(0.34);
  d.toggleGrow("Plan");
  expect(d.heightFor("Plan")).toBe(0.34);
});
test("a spec-watcher entry is appended newest first and capped at 50", () => {
  const d = createDockStore(mem());
  for (let i = 0; i < 52; i++) d.addWatcherEntry("task-1", { at: `t${i}`, reason: `r${i}` });
  expect(d.store.get().watcherLog["task-1"]).toHaveLength(50);
  expect(d.store.get().watcherLog["task-1"][0].reason).toBe("r51");
});
test("the store starts from the persisted tabs, heights and position", () => {
  const p = mem();
  p.last = { position: "Right", tabByStage: { Plan: "ticket" }, heightByStage: {}, watcherLog: {} };
  const d = createDockStore(p);
  expect(d.store.get().position).toBe("Right");
  expect(d.tabFor("Plan", false)).toBe("ticket");
});
