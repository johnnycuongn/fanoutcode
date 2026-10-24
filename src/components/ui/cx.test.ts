import { expect, test } from "vitest";
import { cx } from "./cx";

test("cx merges conflicting Tailwind classes, last wins", () => {
  expect(cx("p-2", false && "hidden", "p-3")).toBe("p-3");
});
