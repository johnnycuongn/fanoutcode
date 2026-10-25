// Review notes pinned to lines of the working side (§6.4): a line decoration coloured by severity, and a block widget
// after the line that is only a mount point; the note card itself is a React portal into it (DiffEditor), so no
// markup is ever built from note text here. A note past the end of the file is clamped to the last line, never
// dropped (Review Focus 1).
import { StateEffect, StateField, type Extension, type Range } from "@codemirror/state";
import { Decoration, EditorView, WidgetType, type DecorationSet } from "@codemirror/view";
import type { ReviewNote } from "./types";

export interface PlacedNote {
  note: ReviewNote;
  line: number;
  clamped: boolean;
}

/** 1-based, sorted by line; lines below 1 go to 1 and lines past the end to the last line, marked `clamped`. */
export function placeNotes(notes: ReviewNote[], lineCount: number): PlacedNote[] {
  const last = Math.max(lineCount, 1);
  return notes
    .map((note) => {
      const line = Math.min(Math.max(note.line, 1), last);
      return { note, line, clamped: line !== note.line };
    })
    .sort((a, b) => a.line - b.line);
}

// Widget mount points by note id, for React portals.
let slots: ReadonlyMap<string, HTMLElement> = new Map();
const slotListeners = new Set<(s: ReadonlyMap<string, HTMLElement>) => void>();
function setSlots(next: Map<string, HTMLElement>) {
  slots = next;
  for (const l of [...slotListeners]) l(slots);
}

export const noteSlots = {
  subscribe(fn: (s: ReadonlyMap<string, HTMLElement>) => void): () => void {
    slotListeners.add(fn);
    return () => {
      slotListeners.delete(fn);
    };
  },
  get: (): ReadonlyMap<string, HTMLElement> => slots,
};

class NoteSlot extends WidgetType {
  constructor(readonly id: string) {
    super();
  }
  eq(other: NoteSlot): boolean {
    return other.id === this.id;
  }
  toDOM(): HTMLElement {
    const el = document.createElement("div");
    el.className = "cm-review-slot";
    el.dataset.noteId = this.id;
    setSlots(new Map(slots).set(this.id, el));
    return el;
  }
  destroy(dom: HTMLElement): void {
    if (slots.get(this.id) !== dom) return;
    const next = new Map(slots);
    next.delete(this.id);
    setSlots(next);
  }
  ignoreEvent(): boolean {
    return true;
  }
}

export const setNotes = StateEffect.define<PlacedNote[]>();

export const notesField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(set, tr) {
    let next = set.map(tr.changes);
    for (const e of tr.effects) {
      if (!e.is(setNotes)) continue;
      const ranges: Range<Decoration>[] = [];
      for (const p of e.value) {
        if (p.line > tr.state.doc.lines) continue;
        const line = tr.state.doc.line(p.line);
        const cls = `cm-review-line cm-review-${p.note.severity}${p.note.security ? " cm-review-security" : ""}`;
        ranges.push(Decoration.line({ class: cls }).range(line.from));
        ranges.push(Decoration.widget({ widget: new NoteSlot(p.note.id), block: true, side: 1 }).range(line.to));
      }
      next = Decoration.set(ranges, true);
    }
    return next;
  },
  provide: (f) => EditorView.decorations.from(f),
});

export const reviewTheme: Extension = EditorView.baseTheme({
  ".cm-review-line": { boxShadow: "inset 3px 0 0 var(--color-mute)" },
  ".cm-review-critical, .cm-review-high": {
    boxShadow: "inset 3px 0 0 var(--color-bad)",
    backgroundColor: "color-mix(in oklab, var(--color-bad) 8%, transparent)",
  },
  ".cm-review-medium": {
    boxShadow: "inset 3px 0 0 var(--color-warn)",
    backgroundColor: "color-mix(in oklab, var(--color-warn) 8%, transparent)",
  },
  ".cm-review-slot": { padding: "4px 8px 6px" },
});
