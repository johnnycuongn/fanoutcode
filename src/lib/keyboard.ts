// The one keyboard registry (spec §10.5). Every ⌘ chord the app claims is in KEYMAP; the terminal gets every other
// key. One capture-phase keydown listener (installKeyboard) matches a chord against the focused scope and calls the
// handlers registered for it.

export type Chord = { key: string; meta: boolean; shift: boolean; alt: boolean; ctrl: boolean };

export type ShortcutId =
  | "window.next"
  | "tab.nth"
  | "board"
  | "view.prev"
  | "view.next"
  | "view.new"
  | "task.new"
  | "palette"
  | "pane.pick"
  | "sidebar.toggle"
  | "pane.splitRight"
  | "pane.splitDown"
  | "pane.close"
  | "pane.focusLeft"
  | "pane.focusUp"
  | "pane.focusRight"
  | "pane.focusDown"
  | "dock.toggle"
  | "team.grow"
  | "task.primary"
  | "task.rename"
  | "task.archive"
  | "terminalStrip.toggle"
  | "workspace.save"
  | "settings";

/**
 * `global` chords match everywhere; `pane` chords in a pane or a Spec page; `spec-page` chords only on a Spec page;
 * `sidebar` chords only while focus is in the sidebar (so ⌘S stays with the diff and memory editors elsewhere).
 */
export type Scope = "global" | "pane" | "spec-page" | "sidebar";

export type ActiveScope = { scope: Scope; terminalFocused: boolean; sidebarFocused?: boolean };

const DIGITS = ["⌘1", "⌘2", "⌘3", "⌘4", "⌘5", "⌘6", "⌘7", "⌘8", "⌘9"];

export const KEYMAP: Record<ShortcutId, { chords: string[]; scope: Scope }> = {
  "window.next": { chords: ["⌘`"], scope: "global" },
  "tab.nth": { chords: DIGITS, scope: "global" },
  board: { chords: ["⌘B"], scope: "global" },
  "view.prev": { chords: ["⌘⇧["], scope: "global" },
  "view.next": { chords: ["⌘⇧]"], scope: "global" },
  "view.new": { chords: ["⌘N"], scope: "global" },
  // Global, not pane: ⌘T starts a task from the Board too (Ruling in the plan 06 report).
  "task.new": { chords: ["⌘T"], scope: "global" },
  palette: { chords: ["⌘K"], scope: "global" },
  "pane.pick": { chords: ["⌘P"], scope: "pane" },
  "sidebar.toggle": { chords: ["⌘\\"], scope: "global" },
  "pane.splitRight": { chords: ["⌘D"], scope: "pane" },
  "pane.splitDown": { chords: ["⌘⇧D"], scope: "pane" },
  "pane.close": { chords: ["⌘W"], scope: "pane" },
  "pane.focusLeft": { chords: ["⌘⌥←"], scope: "pane" },
  "pane.focusUp": { chords: ["⌘⌥↑"], scope: "pane" },
  "pane.focusRight": { chords: ["⌘⌥→"], scope: "pane" },
  "pane.focusDown": { chords: ["⌘⌥↓"], scope: "pane" },
  "dock.toggle": { chords: ["⌘J"], scope: "pane" },
  "team.grow": { chords: ["⌘⇧J"], scope: "pane" },
  "task.primary": { chords: ["⌘⏎"], scope: "pane" },
  "task.rename": { chords: ["⌘E"], scope: "pane" },
  "task.archive": { chords: ["⌘⇧A"], scope: "pane" },
  "terminalStrip.toggle": { chords: ["⌃`"], scope: "spec-page" },
  // §4.2: a workspace file saved outside app data is written on ⌘S with the sidebar focused
  "workspace.save": { chords: ["⌘S"], scope: "sidebar" },
  settings: { chords: ["⌘,"], scope: "global" },
};

/** Chords the terminal keeps; the conflict test fails if KEYMAP ever claims one. */
export const TERMINAL_RESERVED: string[] = ["⌘C", "⌘V", "⌘A", "⌘F", "⌘+", "⌘-", "⌘0"];

const MODIFIER_GLYPHS: Record<string, keyof Omit<Chord, "key">> = {
  "⌘": "meta",
  "⇧": "shift",
  "⌥": "alt",
  "⌃": "ctrl",
};
const KEY_GLYPHS: Record<string, string> = {
  "⏎": "Enter",
  "←": "ArrowLeft",
  "↑": "ArrowUp",
  "→": "ArrowRight",
  "↓": "ArrowDown",
};
// With Shift held, e.key reports the shifted symbol; the map's labels name the unshifted key.
const UNSHIFTED: Record<string, string> = { "{": "[", "}": "]", "~": "`", "|": "\\", "<": ",", _: "-" };

const normKey = (key: string) => (key.length === 1 ? (UNSHIFTED[key] ?? key).toLowerCase() : key);

export function parseChord(label: string): Chord {
  const chord: Chord = { key: "", meta: false, shift: false, alt: false, ctrl: false };
  const chars = [...label];
  let i = 0;
  while (i < chars.length - 1 && MODIFIER_GLYPHS[chars[i]]) chord[MODIFIER_GLYPHS[chars[i++]]] = true;
  const rest = chars.slice(i).join("");
  if (!rest) throw new Error(`Chord "${label}" has no key`);
  chord.key = KEY_GLYPHS[rest] ?? rest;
  return chord;
}

export function chordOf(e: KeyboardEvent): Chord {
  return { key: e.key, meta: e.metaKey, shift: e.shiftKey, alt: e.altKey, ctrl: e.ctrlKey };
}

export function chordEquals(a: Chord, b: Chord): boolean {
  return (
    normKey(a.key) === normKey(b.key) &&
    a.meta === b.meta &&
    a.shift === b.shift &&
    a.alt === b.alt &&
    a.ctrl === b.ctrl
  );
}

/** "⌘⇧[" → "cmd+shift+[", the form the KeyHint primitive renders. */
export function chordHint(label: string): string {
  const c = parseChord(label);
  const key = { Enter: "enter", ArrowLeft: "left", ArrowUp: "up", ArrowRight: "right", ArrowDown: "down" }[c.key];
  return [c.meta && "cmd", c.ctrl && "ctrl", c.alt && "alt", c.shift && "shift", key ?? c.key.toLowerCase()]
    .filter(Boolean)
    .join("+");
}

const PARSED: Array<{ id: ShortcutId; chord: Chord; label: string; scope: Scope }> = (
  Object.entries(KEYMAP) as Array<[ShortcutId, { chords: string[]; scope: Scope }]>
).flatMap(([id, { chords, scope }]) => chords.map((label) => ({ id, chord: parseChord(label), label, scope })));

function scopeAllows(chordScope: Scope, active: ActiveScope): boolean {
  if (chordScope === "global") return true;
  if (chordScope === "sidebar") return active.sidebarFocused === true;
  if (chordScope === "pane") return active.scope === "pane" || active.scope === "spec-page";
  return active.scope === "spec-page";
}

type Handler = (m: { digit?: number }) => void;
const handlers = new Map<ShortcutId, Set<Handler>>();

export function registerHandler(id: ShortcutId, fn: Handler): () => void {
  const set = handlers.get(id) ?? new Set<Handler>();
  handlers.set(id, set);
  set.add(fn);
  return () => {
    set.delete(fn);
  };
}

export function matchShortcut(
  e: KeyboardEvent,
  active: ActiveScope,
  opts: { dispatch?: boolean } = {},
): { id: ShortcutId; digit?: number } | null {
  const chord = chordOf(e);
  // A focused terminal keeps every key that is not a ⌘ chord (⌃` included).
  if (active.terminalFocused && !chord.meta) return null;
  const hit = PARSED.find((p) => scopeAllows(p.scope, active) && chordEquals(p.chord, chord));
  if (!hit) return null;
  const match: { id: ShortcutId; digit?: number } =
    hit.id === "tab.nth" ? { id: hit.id, digit: Number(hit.chord.key) } : { id: hit.id };
  if (opts.dispatch) for (const fn of [...(handlers.get(hit.id) ?? [])]) fn({ digit: match.digit });
  return match;
}

/** True for every chord the app claims; the xterm pane hands these back to the window (Plan 04). */
export function isAppChord(e: KeyboardEvent): boolean {
  return matchShortcut(e, { scope: "spec-page", terminalFocused: true }) !== null;
}

/** One capture-phase keydown listener for the window; returns the uninstall. */
export function installKeyboard(target: Window, getActive: () => ActiveScope): () => void {
  const onKeyDown = (e: KeyboardEvent) => {
    if (matchShortcut(e, getActive(), { dispatch: true })) e.preventDefault();
  };
  target.addEventListener("keydown", onKeyDown, true);
  return () => target.removeEventListener("keydown", onKeyDown, true);
}
