// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import {
  SWATCHES,
  applyAccent,
  applyTheme,
  readStoredTheme,
  resolveTheme,
  setThemePreference,
  startThemeSync,
  storeTheme,
  swatchClass,
} from "./theme";

describe("resolveTheme", () => {
  it("system follows the OS, explicit choices ignore it", () => {
    expect(resolveTheme("System", true)).toBe("dark");
    expect(resolveTheme("System", false)).toBe("light");
    expect(resolveTheme("Light", true)).toBe("light");
    expect(resolveTheme("Dark", false)).toBe("dark");
  });
});

describe("applyTheme", () => {
  it("writes data-theme on <html>", () => {
    applyTheme("Dark", false);
    expect(document.documentElement.dataset.theme).toBe("dark");
    applyTheme("System", false);
    expect(document.documentElement.dataset.theme).toBe("light");
  });
});

describe("stored preference", () => {
  it("defaults to system when storage is empty, unknown or throwing", () => {
    localStorage.clear();
    expect(readStoredTheme()).toBe("System");
    localStorage.setItem("fanoutcode.theme", "sepia");
    expect(readStoredTheme()).toBe("System");
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("private mode");
    });
    expect(readStoredTheme()).toBe("System");
    vi.restoreAllMocks();
  });
  it("round-trips a choice", () => {
    storeTheme("Dark");
    expect(readStoredTheme()).toBe("Dark");
  });
});

describe("applyAccent", () => {
  it("sets data-accent for presets and clears it for blue", () => {
    applyAccent("teal");
    expect(document.documentElement.dataset.accent).toBe("teal");
    applyAccent("blue");
    expect(document.documentElement.dataset.accent).toBeUndefined();
  });
});

describe("swatchClass", () => {
  it("maps every swatch and part to a literal token class, with slate as the fallback", () => {
    expect(SWATCHES).toHaveLength(10);
    expect(swatchClass("violet", "bg")).toBe("bg-swatch-violet");
    expect(swatchClass("rose", "text")).toBe("text-swatch-rose");
    expect(swatchClass("cyan", "edge")).toBe("border-swatch-cyan");
    expect(swatchClass("teal", "bg")).toBe("bg-swatch-slate");
  });
  it("names only swatches that styles.css defines", () => {
    const css = readFileSync("src/styles.css", "utf8");
    for (const s of SWATCHES) expect(css).toContain(`--color-swatch-${s}:`);
  });
});

describe("startThemeSync", () => {
  it("follows OS changes while system, stops following once explicit, and unsubscribes", () => {
    type Listener = (e: { matches: boolean }) => void;
    const listeners = new Set<Listener>();
    let matches = false;
    vi.stubGlobal("matchMedia", () => ({
      get matches() {
        return matches;
      },
      addEventListener: (_: string, l: Listener) => listeners.add(l),
      removeEventListener: (_: string, l: Listener) => listeners.delete(l),
    }));
    storeTheme("System");
    const stop = startThemeSync();
    expect(document.documentElement.dataset.theme).toBe("light");
    matches = true;
    listeners.forEach((l) => l({ matches }));
    expect(document.documentElement.dataset.theme).toBe("dark");
    setThemePreference("Light");
    matches = false;
    listeners.forEach((l) => l({ matches }));
    expect(document.documentElement.dataset.theme).toBe("light");
    matches = true;
    listeners.forEach((l) => l({ matches }));
    expect(document.documentElement.dataset.theme).toBe("light");
    stop();
    expect(listeners.size).toBe(0);
    vi.unstubAllGlobals();
  });
});
