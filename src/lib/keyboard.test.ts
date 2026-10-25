import { describe, it, expect } from "vitest";
import { KEYMAP, TERMINAL_RESERVED, parseChord, chordEquals, matchShortcut, registerHandler } from "./keyboard";

const ev = (key: string, m: Partial<KeyboardEventInit> = {}) =>
  new KeyboardEvent("keydown", { key, metaKey: false, shiftKey: false, altKey: false, ctrlKey: false, ...m });

describe("keyboard registry", () => {
  it("parses every label in the map", () => {
    for (const { chords } of Object.values(KEYMAP)) for (const c of chords) expect(() => parseChord(c)).not.toThrow();
    expect(parseChord("⌘⇧[")).toEqual({ key: "[", meta: true, shift: true, alt: false, ctrl: false });
    expect(parseChord("⌘⏎")).toEqual({ key: "Enter", meta: true, shift: false, alt: false, ctrl: false });
    expect(parseChord("⌘⌥←").key).toBe("ArrowLeft");
  });
  it("claims only ⌘ chords except the Spec strip toggle", () => {
    for (const [id, { chords, scope }] of Object.entries(KEYMAP))
      for (const c of chords) {
        const chord = parseChord(c);
        if (id === "terminalStrip.toggle") {
          expect(chord.ctrl).toBe(true);
          expect(scope).toBe("spec-page");
        } else expect(chord.meta).toBe(true);
      }
  });
  it("has no duplicate chords and none reserved by the terminal", () => {
    const all = Object.values(KEYMAP).flatMap((k) => k.chords.map(parseChord));
    for (let i = 0; i < all.length; i++)
      for (let j = i + 1; j < all.length; j++) expect(chordEquals(all[i], all[j])).toBe(false);
    for (const r of TERMINAL_RESERVED.map(parseChord)) for (const c of all) expect(chordEquals(r, c)).toBe(false);
  });
  it("matches ⌘3 as tab.nth with digit 3 and ⌘⇧] as view.next", () => {
    expect(matchShortcut(ev("3", { metaKey: true }), { scope: "global", terminalFocused: false })).toEqual({
      id: "tab.nth",
      digit: 3,
    });
    expect(
      matchShortcut(ev("]", { metaKey: true, shiftKey: true }), { scope: "global", terminalFocused: false }),
    ).toEqual({ id: "view.next" });
  });
  it("never claims ⌃` while a terminal is focused, and never claims ⌘C or plain keys", () => {
    expect(matchShortcut(ev("`", { ctrlKey: true }), { scope: "spec-page", terminalFocused: true })).toBeNull();
    expect(matchShortcut(ev("`", { ctrlKey: true }), { scope: "spec-page", terminalFocused: false })).toEqual({
      id: "terminalStrip.toggle",
    });
    expect(matchShortcut(ev("c", { metaKey: true }), { scope: "pane", terminalFocused: true })).toBeNull();
    expect(matchShortcut(ev("a"), { scope: "pane", terminalFocused: true })).toBeNull();
  });
  it("does not match pane-scoped chords in global scope", () => {
    expect(matchShortcut(ev("d", { metaKey: true }), { scope: "global", terminalFocused: false })).toBeNull();
    expect(matchShortcut(ev("d", { metaKey: true }), { scope: "pane", terminalFocused: true })).toEqual({
      id: "pane.splitRight",
    });
  });
  it("calls registered handlers and unregisters", () => {
    const seen: number[] = [];
    const off = registerHandler("tab.nth", ({ digit }) => seen.push(digit!));
    matchShortcut(ev("2", { metaKey: true }), { scope: "global", terminalFocused: false }, { dispatch: true });
    off();
    matchShortcut(ev("2", { metaKey: true }), { scope: "global", terminalFocused: false }, { dispatch: true });
    expect(seen).toEqual([2]);
  });
  it("claims ⌘S for the workspace file only while the sidebar has focus", () => {
    const save = ev("s", { metaKey: true });
    expect(matchShortcut(save, { scope: "pane", terminalFocused: false })).toBeNull();
    expect(matchShortcut(save, { scope: "global", terminalFocused: false, sidebarFocused: true })).toEqual({
      id: "workspace.save",
    });
  });
});
