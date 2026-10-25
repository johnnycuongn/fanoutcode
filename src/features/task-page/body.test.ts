import { expect, test } from "vitest";
import { bodyFor, wantsSession } from "./body";
import { makeTask } from "./testing/fixtures";

test("bodyFor", () => {
  expect(bodyFor(makeTask({ stage: "Spec" }))).toBe("spec");
  expect(bodyFor(makeTask({ stage: "Plan" }))).toBe("terminal");
  expect(bodyFor(makeTask({ stage: "Plan", primary_session: null }))).toBe("no-session");
});
test("the page opens a session for Plan through Commit only, never for an archived task", () => {
  expect(
    ["Spec", "Plan", "Build", "Review", "Test", "Commit", "Done"].map((s) =>
      wantsSession(makeTask({ stage: s as never })),
    ),
  ).toEqual([false, true, true, true, true, true, false]);
  expect(wantsSession(makeTask({ archived: true }))).toBe(false);
});
