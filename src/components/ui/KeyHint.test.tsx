import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { KeyHint, chordLabel, formatChord } from "./KeyHint";

test.each([
  ["cmd+shift+a", "⌘⇧A"],
  ["shift+cmd+a", "⌘⇧A"],
  ["cmd+enter", "⌘⏎"],
  ["ctrl+`", "⌃`"],
  ["cmd+alt+left", "⌘⌥←"],
  ["cmd+shift+[", "⌘⇧["],
  ["cmd+,", "⌘,"],
  ["cmd+1", "⌘1"],
  ["esc", "⎋"],
])("formatChord(%s) → %s", (chord, glyphs) => expect(formatChord(chord)).toBe(glyphs));

test("chordLabel spells the chord out", () => expect(chordLabel("cmd+shift+a")).toBe("Command Shift A"));

test("renders a kbd with the glyphs and a spoken label", () => {
  render(<KeyHint keys="cmd+k" />);
  const kbd = screen.getByText("⌘K");
  expect(kbd.tagName).toBe("KBD");
  expect(kbd).toHaveAttribute("aria-label", "Command K");
});
