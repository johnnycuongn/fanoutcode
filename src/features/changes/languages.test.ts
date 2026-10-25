import { expect, test } from "vitest";
import { barWidths } from "./files";
import { languageFor } from "./languages";

test.each([
  [3, 1, 4, { add: 75, del: 25 }],
  [0, 0, 4, { add: 0, del: 0 }],
  [4, 0, 0, { add: 0, del: 0 }],
])("barWidths(%i, %i, %i)", (a, r, m, out) => expect(barWidths(a, r, m)).toEqual(out));
test("languageFor resolves TypeScript for .ts and null for an unknown extension", async () => {
  expect(await languageFor("src/a.ts")).not.toBeNull();
  expect(await languageFor("notes.unknownext")).toBeNull();
});
