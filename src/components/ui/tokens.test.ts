import { expect, test } from "vitest";
import { STAGES, statusLabel, statusTone, stageClasses, swatchClasses, toneClasses } from "./tokens";
import { SWATCHES } from "@/lib/theme";

test("STAGES lists the seven stages in pipeline order", () => {
  expect(STAGES).toEqual(["spec", "plan", "build", "review", "test", "commit", "done"]);
});

test("statusTone maps working, needs-you and idle to ok, warn and neutral", () => {
  expect(statusTone("working")).toBe("ok");
  expect(statusTone("needs-you")).toBe("warn");
  expect(statusTone("idle")).toBe("neutral");
});

test("statusLabel gives the chip copy", () => {
  expect(statusLabel("working")).toBe("Working");
  expect(statusLabel("needs-you")).toBe("Needs you");
  expect(statusLabel("idle")).toBe("Idle");
});

test("every tone, stage and swatch has soft, solid and text classes with no hex colour", () => {
  for (const map of [toneClasses, stageClasses, swatchClasses]) {
    for (const fill of Object.values(map)) {
      for (const key of ["soft", "solid", "text"] as const) {
        expect(fill[key]).toBeTruthy();
        expect(fill[key]).not.toMatch(/#[0-9a-fA-F]{3,8}/);
      }
    }
  }
  expect(Object.keys(stageClasses)).toEqual([...STAGES]);
});

test("swatchClasses covers exactly the shared swatch palette", () => {
  expect(Object.keys(swatchClasses)).toEqual([...SWATCHES]);
});
