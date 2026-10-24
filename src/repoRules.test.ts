import { existsSync, readFileSync } from "node:fs";
import { expect, it } from "vitest";

it("keeps CLAUDE.md under 150 lines and every rule path-scoped", () => {
  expect(readFileSync("CLAUDE.md", "utf8").split("\n").length).toBeLessThan(150);
  for (const f of ["ui", "ipc", "core", "prompts"])
    expect(readFileSync(`.claude/rules/${f}.md`, "utf8")).toMatch(/^---\npaths:/);
  for (const adr of ["0001-generated-bindings", "0002-settings-in-sqlite"])
    expect(existsSync(`docs/adr/${adr}.md`)).toBe(true);
});
