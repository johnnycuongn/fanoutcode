// @vitest-environment jsdom
/* eslint-disable teamcode/no-colour-literals -- the test sets CSS variables the way styles.css does */
import { describe, it, expect } from "vitest";
import { readXtermTheme, rendererFor, resolvePalette } from "./terminalTheme";

describe("resolvePalette", () => {
  it("follows the app theme by default", () => {
    expect(resolvePalette({ type: "FollowApp" }, "dark")).toBe("graphite");
    expect(resolvePalette({ type: "FollowApp" }, "light")).toBe("soft-white");
  });
  it("forces dark or light regardless of the app", () => {
    expect(resolvePalette({ type: "AlwaysDark" }, "light")).toBe("graphite");
    expect(resolvePalette({ type: "AlwaysLight" }, "dark")).toBe("soft-white");
  });
  it("maps a named palette", () => {
    expect(resolvePalette({ type: "Named", palette: "SolarizedDark" }, "light")).toBe("solarized-dark");
  });
});

describe("readXtermTheme", () => {
  it("builds the xterm theme from --term-* variables", () => {
    const el = document.createElement("div");
    el.style.setProperty("--term-bg", "#111318");
    el.style.setProperty("--term-ansi-1", "#ff0000");
    const t = readXtermTheme(el);
    expect(t.background).toBe("#111318");
    expect(t.red).toBe("#ff0000");
  });
});

it("ligatures switch the renderer to dom", () => {
  expect(rendererFor(true)).toBe("dom");
  expect(rendererFor(false)).toBe("webgl");
});
