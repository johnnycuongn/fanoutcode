import type { ComponentProps, ReactElement } from "react";
import { cx } from "./cx";

// Chord syntax: tokens joined by "+" ("cmd+shift+a"). Output order is ⌘ ⌃ ⌥ ⇧ then the key (the spec's copy).
const MODIFIERS: Record<string, { glyph: string; name: string; rank: number }> = {
  cmd: { glyph: "⌘", name: "Command", rank: 0 },
  mod: { glyph: "⌘", name: "Command", rank: 0 },
  meta: { glyph: "⌘", name: "Command", rank: 0 },
  ctrl: { glyph: "⌃", name: "Control", rank: 1 },
  control: { glyph: "⌃", name: "Control", rank: 1 },
  alt: { glyph: "⌥", name: "Option", rank: 2 },
  opt: { glyph: "⌥", name: "Option", rank: 2 },
  option: { glyph: "⌥", name: "Option", rank: 2 },
  shift: { glyph: "⇧", name: "Shift", rank: 3 },
};

const KEYS: Record<string, { glyph: string; name: string }> = {
  enter: { glyph: "⏎", name: "Return" },
  return: { glyph: "⏎", name: "Return" },
  esc: { glyph: "⎋", name: "Escape" },
  escape: { glyph: "⎋", name: "Escape" },
  tab: { glyph: "⇥", name: "Tab" },
  space: { glyph: "␣", name: "Space" },
  backspace: { glyph: "⌫", name: "Backspace" },
  delete: { glyph: "⌦", name: "Delete" },
  up: { glyph: "↑", name: "Up Arrow" },
  down: { glyph: "↓", name: "Down Arrow" },
  left: { glyph: "←", name: "Left Arrow" },
  right: { glyph: "→", name: "Right Arrow" },
};

function parse(chord: string): { mods: { glyph: string; name: string }[]; key: { glyph: string; name: string }[] } {
  const mods: { glyph: string; name: string; rank: number }[] = [];
  const key: { glyph: string; name: string }[] = [];
  for (const raw of chord.split("+")) {
    const token = raw.trim().toLowerCase();
    if (!token) continue;
    const mod = MODIFIERS[token];
    if (mod) {
      if (!mods.some((m) => m.rank === mod.rank)) mods.push(mod);
      continue;
    }
    key.push(KEYS[token] ?? { glyph: token.toUpperCase(), name: token.toUpperCase() });
  }
  return { mods: mods.sort((a, b) => a.rank - b.rank), key };
}

/** 'cmd+shift+a' → '⌘⇧A' */
export function formatChord(chord: string): string {
  const { mods, key } = parse(chord);
  return [...mods, ...key].map((p) => p.glyph).join("");
}

/** 'cmd+shift+a' → 'Command Shift A' (the spoken label). */
export function chordLabel(chord: string): string {
  const { mods, key } = parse(chord);
  return [...mods, ...key].map((p) => p.name).join(" ");
}

export type KeyHintProps = ComponentProps<"kbd"> & { keys: string; size?: "sm" | "md" };

export function KeyHint({ keys, size = "md", className, ...props }: KeyHintProps): ReactElement {
  return (
    <kbd
      aria-label={chordLabel(keys)}
      className={cx("font-mono text-mute", size === "sm" ? "text-[10px]" : "text-xs", className)}
      {...props}
    >
      {formatChord(keys)}
    </kbd>
  );
}
