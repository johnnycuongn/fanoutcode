import { expect, test } from "vitest";
import { notesForFile, parseReviewNotes, splitLocation, unplacedNotes } from "./reviewNotes";

const md = `# Review\n\n## security reviewer\n- api/src/auth.ts:42 · high · Token compared with ==\n  Use a constant-time compare.\n\n## best-practice reviewer\n- src/App.tsx:7 · low · Unused import\n* web/src/x.ts:3 · shout · odd severity\nNot an entry line.\n`;
test("parses reviewer headings, locations, severities and continuation lines", () => {
  const notes = parseReviewNotes(md);
  expect(notes).toHaveLength(3);
  expect(notes[0]).toMatchObject({
    reviewer: "security reviewer",
    security: true,
    location: "api/src/auth.ts",
    line: 42,
    severity: "high",
    text: "Token compared with ==\nUse a constant-time compare.",
  });
  expect(notes[1]).toMatchObject({
    reviewer: "best-practice reviewer",
    security: false,
    location: "src/App.tsx",
    line: 7,
    severity: "low",
  });
});
test("an unknown severity becomes info and ids are unique", () => {
  const notes = parseReviewNotes(md);
  expect(notes[2].severity).toBe("info");
  expect(new Set(notes.map((n) => n.id)).size).toBe(3);
});
test("splitLocation uses the first segment as the repo only when it names a known repo", () => {
  expect(splitLocation("api/src/auth.ts", ["api", "web"])).toEqual({ repo: "api", path: "src/auth.ts" });
  expect(splitLocation("src/App.tsx", ["api", "web"])).toEqual({ repo: null, path: "src/App.tsx" });
});
test("notesForFile and unplacedNotes agree on placement", () => {
  const notes = parseReviewNotes(md);
  const repos = ["api", "web"];
  expect(notesForFile(notes, repos, { repo: "api", home: true, path: "src/auth.ts" })).toHaveLength(1);
  expect(notesForFile(notes, repos, { repo: "api", home: true, path: "src/App.tsx" })).toHaveLength(1); // unprefixed → home repo
  expect(notesForFile(notes, repos, { repo: "web", home: false, path: "src/App.tsx" })).toHaveLength(0);
  expect(
    unplacedNotes(notes, repos, [
      { repo: "api", home: true, path: "src/auth.ts" },
      { repo: "api", home: true, path: "src/App.tsx" },
    ]).map((n) => n.location),
  ).toEqual(["web/src/x.ts"]);
});
test("an empty or heading-only file yields no notes", () => {
  expect(parseReviewNotes("")).toEqual([]);
  expect(parseReviewNotes("## reviewer\n")).toEqual([]);
});
test("reads the Review prompt's own form: backticks, blocker to nit, and a Summary that is not a note", () => {
  const notes = parseReviewNotes(
    "## security-reviewer\n`api/src/a.ts:3 · blocker · Secret in the log`\n- web/b.ts:9 · major · x\n- web/c.ts:1 · minor · y\n- web/d.ts:2 · nit · z\n## Summary\n- Does what the spec asks.\n",
  );
  expect(notes.map((n) => [n.location, n.severity, n.security])).toEqual([
    ["api/src/a.ts", "critical", true],
    ["web/b.ts", "high", true],
    ["web/c.ts", "medium", true],
    ["web/d.ts", "low", true],
  ]);
});
