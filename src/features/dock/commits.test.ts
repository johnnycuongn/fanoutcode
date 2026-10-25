import { expect, test } from "vitest";
import { commitSummary, draftsFrom } from "./commits";
import { drafts } from "./testing/fakes";

test("draftsFrom yields one draft per drafted commit under its repo name", () => {
  expect(draftsFrom(drafts)).toEqual([
    {
      repo: "api",
      message: "feat(auth): constant-time token compare\n\nCloses TC-12.",
      files: ["src/auth.ts", "src/auth.test.ts"],
    },
    { repo: "api", message: "chore: bump deps", files: [] },
    { repo: "web", message: "fix(dock): remember tab per stage", files: ["src/features/dock/store.ts"] },
  ]);
});
test("draftsFrom tolerates no drafts", () => expect(draftsFrom([])).toEqual([]));
test("commitSummary counts files and repos with singular forms", () => {
  expect(commitSummary(draftsFrom(drafts))).toBe("Commit 3 files in 2 repos");
  expect(commitSummary([{ repo: "api", message: "x", files: ["a"] }])).toBe("Commit 1 file in 1 repo");
});
