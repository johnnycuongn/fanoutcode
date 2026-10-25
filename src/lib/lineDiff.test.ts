import { expect, test } from "vitest";
import { diffStats, lineDiff } from "./lineDiff";

test("marks added, removed and kept lines in order", () => {
  expect(lineDiff("a\nb\nc", "a\nx\nc")).toEqual([
    { kind: "same", text: "a" },
    { kind: "del", text: "b" },
    { kind: "add", text: "x" },
    { kind: "same", text: "c" },
  ]);
});

test("an empty side is all additions or all removals", () => {
  expect(lineDiff("", "one\ntwo")).toEqual([
    { kind: "add", text: "one" },
    { kind: "add", text: "two" },
  ]);
  expect(lineDiff("one\n", "")).toEqual([{ kind: "del", text: "one" }]);
});

test("counts additions and removals", () => {
  expect(diffStats(lineDiff("a\nb\nc\n", "a\nc\nd\ne\n"))).toEqual({ added: 2, removed: 1 });
});

test("a very large middle falls back to removing then adding it", () => {
  const before = Array.from({ length: 3000 }, (_, i) => `b${i}`).join("\n");
  const after = Array.from({ length: 3000 }, (_, i) => `a${i}`).join("\n");
  const lines = lineDiff(before, after);
  expect(diffStats(lines)).toEqual({ added: 3000, removed: 3000 });
  expect(lines[0].kind).toBe("del");
});
