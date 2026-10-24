// @vitest-environment node
// Pins the lint boundary from the primitives' side (the rules are Plan 01's): raw controls only inside this folder,
// colours only in the theme file, and a primitive never reaches into features, bindings or stores.
import { readdirSync, readFileSync } from "node:fs";
import { ESLint } from "eslint";
import { expect, test } from "vitest";

const eslint = new ESLint({ cwd: process.cwd() });
const src = 'export const Probe = () => <button type="button">x</button>;';

test("a raw <button> outside src/components/ui is a lint error", async () => {
  const [r] = await eslint.lintText(src, { filePath: "src/features/probe/Probe.tsx" });
  expect(r!.messages.some((m) => m.severity === 2 && /button/i.test(m.message))).toBe(true);
});

test("the same element inside src/components/ui passes", async () => {
  const [r] = await eslint.lintText(src, { filePath: "src/components/ui/Probe.tsx" });
  expect(r!.messages.filter((m) => /button/i.test(m.message))).toHaveLength(0);
});

test("a hex colour outside the theme file is a lint error", async () => {
  // Built from parts so this file carries no hex literal itself.
  const hex = "#" + "fff";
  const [r] = await eslint.lintText(`export const c = <div className="bg-[${hex}]" />;`, {
    filePath: "src/features/probe/Probe.tsx",
  });
  expect(r!.messages.some((m) => m.severity === 2 && /colou?r/i.test(m.message))).toBe(true);
});

test("every primitive source file lints clean", async () => {
  const results = await eslint.lintFiles(["src/components/ui/**/*.{ts,tsx}"]);
  const problems = results.flatMap((r) => r.messages.map((m) => `${r.filePath}: ${m.message}`));
  expect(problems).toEqual([]);
});

test("no primitive imports features, bindings or a store", () => {
  const dir = "src/components/ui";
  const offenders = readdirSync(dir)
    .filter((f) => /\.tsx?$/.test(f) && !f.includes(".test."))
    .filter((f) =>
      /from\s+["'](@\/features|@\/lib\/bindings|@\/lib\/stores|\.\.\/)/.test(readFileSync(`${dir}/${f}`, "utf8")),
    );
  expect(offenders).toEqual([]);
});
