import { expect, test } from "vitest";
import type { Stage } from "@/lib/bindings";
import { defaultTabFor, entryState, resolveTab, visibleTabs } from "./tabRules";
import type { DockTab } from "./types";

test.each<[Stage, boolean, DockTab[]]>([
  ["Plan", false, ["memory", "team", "changes", "ticket"]],
  ["Review", true, ["memory", "team", "changes", "ticket", "test"]],
  ["Commit", false, ["memory", "team", "changes", "ticket", "commit"]],
  ["Commit", true, ["memory", "team", "changes", "ticket", "test", "commit"]],
])("visibleTabs(%s, %s)", (stage, t, out) => expect(visibleTabs(stage, t)).toEqual(out));
test.each<[Stage, DockTab]>([
  ["Spec", "memory"],
  ["Plan", "memory"],
  ["Build", "team"],
  ["Review", "changes"],
  ["Test", "test"],
  ["Commit", "commit"],
  ["Done", "memory"],
])("defaultTabFor(%s)", (s, t) => expect(defaultTabFor(s)).toBe(t));
test("resolveTab falls back to the default when the remembered tab is hidden, then to the first visible", () => {
  expect(resolveTab("Review", "test", visibleTabs("Review", false))).toBe("changes");
  expect(resolveTab("Test", undefined, visibleTabs("Test", false))).toBe("memory");
  expect(resolveTab("Plan", "ticket", visibleTabs("Plan", false))).toBe("ticket");
});
test("entryState opens Plan…Commit, grows only Build, closes Spec and leaves Done alone", () => {
  expect(entryState("Spec")).toEqual({ open: false, grown: false });
  expect(entryState("Build")).toEqual({ open: true, grown: true });
  expect(entryState("Review")).toEqual({ open: true, grown: false });
  expect(entryState("Done")).toEqual({ open: null, grown: null });
});
