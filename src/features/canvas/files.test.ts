import { describe, it, expect } from "vitest";
import { editDelta, attributeStep, deriveEdits, deriveFiles } from "./files";
import { deriveAgents } from "./attribution";
import { fx, sampleBuild, samplePlan, MEMORY_DIR } from "./fixtures";

const build = () => {
  const events = sampleBuild();
  return deriveEdits(events, MEMORY_DIR, deriveAgents(events), samplePlan());
};

describe("editDelta", () => {
  it("counts lines for Edit, Write and MultiEdit", () => {
    expect(editDelta("Edit", { old_string: "a\nb", new_string: "a\nb\nc" })).toEqual({ added: 3, removed: 2 });
    expect(editDelta("Write", { content: "x\ny" })).toEqual({ added: 2, removed: 0 });
    expect(
      editDelta("MultiEdit", {
        edits: [
          { old_string: "a", new_string: "" },
          { old_string: "", new_string: "b\nc" },
        ],
      }),
    ).toEqual({ added: 2, removed: 1 });
    expect(editDelta("Bash", { command: "ls" })).toEqual({ added: 0, removed: 0 });
  });
});

describe("attributeStep", () => {
  const plan = samplePlan();
  it("uses the single dispatched step", () => expect(attributeStep([1], "/repo/app/src/other.ts", plan)).toBe(1));
  it("picks the dispatched step that lists the file", () =>
    expect(attributeStep([2, 4], "/repo/app/src/toolbar.tsx", plan)).toBe(2));
  it("falls back to the plan step listing the file for the orchestrator", () =>
    expect(attributeStep([], "/repo/app/README.md", plan)).toBe(5));
  it("returns null when nothing matches", () => expect(attributeStep([], "/tmp/scratch.txt", plan)).toBeNull());
});

describe("deriveEdits", () => {
  it("ignores edits inside the memory folder", () => {
    expect(build().some((e) => e.path.endsWith("plan.md"))).toBe(false);
  });
  it("records agent, delta, step and texts per edit", () => {
    expect(build()[0]).toMatchObject({
      seq: 5,
      agentId: "a1",
      path: "/repo/app/src/a.ts",
      added: 2,
      removed: 0,
      step: 1,
      oldText: "",
      newText: "line\nline",
    });
  });
});

describe("deriveFiles", () => {
  it("marks a file shared only when a second distinct agent edits it, at that seq", () => {
    const a = deriveFiles(build()).find((f) => f.path === "/repo/app/src/a.ts")!;
    expect(a.owners).toEqual(["a1", "a2"]);
    expect(a.sharedAt).toBe(9);
    const before = deriveFiles(build().filter((e) => e.seq < 9)).find((f) => f.path === "/repo/app/src/a.ts")!;
    expect(before.sharedAt).toBeNull();
  });
  it("does not mark a file shared when the same agent edits it twice", () => {
    const edits = deriveEdits(
      [fx.edit(1, "a1", "/repo/app/x.ts", "", "a"), fx.edit(2, "a1", "/repo/app/x.ts", "a", "b")],
      MEMORY_DIR,
      [],
      [],
    );
    expect(deriveFiles(edits)[0]).toMatchObject({ owners: ["a1"], sharedAt: null });
  });
  it("prefers git numstat over event deltas for net +/- and unions owners", () => {
    const [f] = deriveFiles(
      build().filter((e) => e.path.endsWith("route.ts")),
      [{ path: "/repo/api/src/route.ts", added: 7, removed: 1, owners: ["a2", "a9"] }],
    );
    expect(f).toMatchObject({ added: 7, removed: 1, owners: ["a2", "a9"], sharedAt: null });
  });
});
