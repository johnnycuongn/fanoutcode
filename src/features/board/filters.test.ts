import { describe, expect, it } from "vitest";
import { applyFilters, countActiveFilters, defaultFilters, isNeedsYou } from "./filters";
import { WORKING, makeTask, needsYou } from "./testing/fixtures";

const ids = (ts: ReturnType<typeof makeTask>[]) => ts.map((t) => t.id);

describe("applyFilters", () => {
  const plain = makeTask({ id: "plain" });
  const hidden = makeTask({ id: "hidden", hidden: true });
  const archived = makeTask({ id: "archived", archived: true });
  const waiting = makeTask({
    id: "waiting",
    state: needsYou({ type: "StageDone" }),
    priority: "High",
    color: "violet",
    jira_key: "TC-12",
    project_id: "p-2",
  });
  const working = makeTask({ id: "working", state: WORKING, priority: "Low", color: "teal" });
  const all = [plain, hidden, archived, waiting, working];

  it("hides archived and hidden tasks by default", () => {
    expect(ids(applyFilters(all, defaultFilters))).toEqual(["plain", "waiting", "working"]);
  });
  it("archived filter shows only archived tasks", () => {
    expect(ids(applyFilters(all, { ...defaultFilters, archived: true }))).toEqual(["archived"]);
  });
  it("showHidden keeps hidden tasks", () => {
    expect(ids(applyFilters(all, { ...defaultFilters, showHidden: true }))).toContain("hidden");
  });
  it("needsYou keeps only tasks waiting on the developer", () => {
    expect(ids(applyFilters(all, { ...defaultFilters, needsYou: true }))).toEqual(["waiting"]);
  });
  it("priority filter keeps the selected priorities only", () => {
    expect(ids(applyFilters(all, { ...defaultFilters, priorities: ["High", "Low"] }))).toEqual(["waiting", "working"]);
  });
  it("colour filter matches the task colour token", () => {
    expect(ids(applyFilters(all, { ...defaultFilters, colors: ["teal"] }))).toEqual(["working"]);
  });
  it("jira key filter matches the key prefix case-insensitively", () => {
    expect(ids(applyFilters(all, { ...defaultFilters, jira: { key: "tc-" } }))).toEqual(["waiting"]);
  });
  it("jira linked keeps tasks with a key and unlinked keeps the rest", () => {
    expect(ids(applyFilters(all, { ...defaultFilters, jira: { linked: true } }))).toEqual(["waiting"]);
    expect(ids(applyFilters(all, { ...defaultFilters, jira: { linked: false } }))).toEqual(["plain", "working"]);
  });
  it("project filter keeps the chosen projects", () => {
    expect(ids(applyFilters(all, { ...defaultFilters, projects: ["p-2"] }))).toEqual(["waiting"]);
  });
});

describe("countActiveFilters", () => {
  it("is 0 for defaults and counts each non-default field once", () => {
    expect(countActiveFilters(defaultFilters)).toBe(0);
    expect(
      countActiveFilters({ ...defaultFilters, needsYou: true, priorities: ["High", "Medium"], jira: { linked: true } }),
    ).toBe(3);
  });
});

describe("isNeedsYou", () => {
  it("is true only for NeedsYou states", () => {
    expect(isNeedsYou({ state: needsYou({ type: "PermissionNeeded" }) })).toBe(true);
    expect(isNeedsYou({ state: WORKING })).toBe(false);
  });
});
