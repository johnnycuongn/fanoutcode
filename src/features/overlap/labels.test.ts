import { describe, expect, it } from "vitest";
import type { OverlapRow, Relation } from "@/lib/bindings";
import { fileRelationLabels, isStash, overlapsText, relationLabel } from "./labels";

function row(relation: string, held = false): OverlapRow {
  return {
    task_id: "task-b2",
    other_task: "task-c3",
    repo_root: "/r/alpha",
    path: "src/a.ts",
    relation: relation as Relation,
    pending: null,
    held,
    noted_out: false,
    noted_in: false,
    decided_at: null,
    updated_at: "2026-10-25T08:00:00Z",
  };
}

describe("relationLabel", () => {
  it.each([
    ["Claimed", "claimed"],
    ["GrantedOnTop", "granted on top"],
    ["TakenOver", "taken over"],
    ["TakenOverBy", "taken over by"],
    ["PlannedBy", "planned by"],
    ["Committed", "committed"],
  ])("labels %s as %s", (relation, label) => {
    expect(relationLabel(row(relation))).toBe(label);
  });
  it("says holding when the row is held", () => {
    expect(relationLabel(row("Claimed", true))).toBe("holding");
  });
  it("falls back to the raw name for an unknown relation", () => {
    expect(relationLabel(row("SomethingNew"))).toBe("SomethingNew");
  });
});

describe("overlapsText", () => {
  it("is singular for one and plural otherwise", () => {
    expect(overlapsText(1)).toBe("1 overlap");
    expect(overlapsText(3)).toBe("3 overlaps");
  });
});

describe("fileRelationLabels", () => {
  it("keeps the distinct labels of the rows on that repo and path", () => {
    const rows = [row("Claimed"), { ...row("Claimed"), other_task: "task-d4" }, row("PlannedBy")];
    rows.push({ ...row("Committed"), repo_root: "/r/beta" }, { ...row("Committed"), path: "src/b.ts" });
    expect(fileRelationLabels(rows, "/r/alpha", "src/a.ts")).toEqual(["claimed", "planned by"]);
    expect(fileRelationLabels(rows, "/r/alpha", "src/z.ts")).toEqual([]);
  });
});

describe("isStash", () => {
  it("reads git stash from the command text, but not its list and show", () => {
    for (const c of ["git stash", "git stash push -u", "git -C /r stash", "ls && git stash pop"])
      expect(isStash(c), c).toBe(true);
    for (const c of ["git stash list", "git stash show", "git commit -m stash", "git reset --hard"])
      expect(isStash(c), c).toBe(false);
  });
});
