import { expect, test } from "vitest";
import type { Project } from "@/lib/bindings";
import { linksChanged } from "./links";
import { makeTask } from "./testing/fixtures";

test("linksChanged is true only when a session exists and the bound roots differ", () => {
  const projects = [
    { id: "proj-1", root: "/a" },
    { id: "proj-2", root: "/b" },
  ] as Project[];
  expect(linksChanged(makeTask({ linked_roots: ["/b"] }), projects)).toBe(false);
  expect(linksChanged(makeTask({ linked_roots: [] }), projects)).toBe(true);
  expect(linksChanged(makeTask({ linked_roots: [], primary_session: null }), projects)).toBe(false);
});
