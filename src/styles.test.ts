/* eslint-disable fanoutcode/no-colour-literals -- this test pins the hex values of the theme file */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { expect, it } from "vitest";

const css = readFileSync("src/styles.css", "utf8");
const HEX = /#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})\b/i; // no `g`: a global regex keeps lastIndex between test() calls

it("carries every spec colour", () => {
  for (const hex of [
    "#f6f6f7",
    "#f0f0f2",
    "#ededef",
    "#ffffff",
    "#e2e2e6",
    "#18181b",
    "#71717a",
    "#3b6cff",
    "#1f9d55",
    "#d97706",
    "#111318",
    "#0d0f13",
    "#161920",
    "#1a1e26",
    "#262b35",
    "#e7e9ee",
    "#8a91a0",
    "#5b8cff",
    "#3ddc84",
    "#f5a524",
    "#7c5cff",
    "#2f6fed",
    "#e8961e",
    "#e0457b",
    "#14b8a6",
    "#8e8e93",
  ])
    expect(css).toContain(hex);
});

it("defines every semantic token and swatch", () => {
  for (const t of ["page", "bar", "well", "card", "line", "ink", "mute", "accent", "ok", "warn", "bad"])
    expect(css).toContain(`--color-${t}:`);
  for (const s of ["spec", "plan", "build", "review", "test", "commit", "done"])
    expect(css).toContain(`--color-stage-${s}:`);
  for (const s of ["rose", "orange", "yellow", "lime", "emerald", "cyan", "indigo", "violet", "fuchsia", "slate"])
    expect(css).toContain(`--color-swatch-${s}:`);
});

it("switches by data-theme, not by media query, and offers the accent presets", () => {
  expect(css).toContain('[data-theme="dark"]');
  expect(css).not.toMatch(/prefers-color-scheme/);
  for (const a of ["violet", "teal", "orange"]) expect(css).toContain(`[data-accent="${a}"]`);
});

it("is the only file under src with a hex colour", () => {
  const walk = (d: string): string[] =>
    readdirSync(d).flatMap((n) => {
      const p = `${d}/${n}`;
      return statSync(p).isDirectory() ? walk(p) : [p];
    });
  const offenders = walk("src").filter(
    (p) => p !== "src/styles.css" && !p.endsWith(".test.ts") && HEX.test(readFileSync(p, "utf8")),
  );
  expect(offenders).toEqual([]);
});
