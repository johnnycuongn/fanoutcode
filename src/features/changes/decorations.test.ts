import { EditorState } from "@codemirror/state";
import { expect, test } from "vitest";
import { notesField, placeNotes, setNotes } from "./decorations";
import type { ReviewNote, Severity } from "./types";

const n = (line: number, id = `x:${line}:0`, severity: Severity = "high"): ReviewNote => ({
  id,
  reviewer: null,
  security: false,
  location: "src/a.ts",
  line,
  severity,
  text: "t",
});
test("placeNotes clamps lines past the end and below one, and sorts", () => {
  expect(placeNotes([n(42), n(0, "z"), n(3)], 10)).toEqual([
    { note: n(0, "z"), line: 1, clamped: true },
    { note: n(3), line: 3, clamped: false },
    { note: n(42), line: 10, clamped: true },
  ]);
});
test("noteDecorations adds one line decoration and one widget at the note's line", () => {
  let state = EditorState.create({ doc: "a\nb\nc", extensions: [notesField] });
  state = state.update({ effects: setNotes.of(placeNotes([n(2)], 3)) }).state;
  const found: { from: number; block: boolean; cls: string | undefined }[] = [];
  state.field(notesField).between(0, state.doc.length, (from, _to, d) => {
    found.push({ from, block: d.spec.block === true, cls: d.spec.class ?? d.spec.attributes?.class });
  });
  expect(found).toEqual([
    { from: state.doc.line(2).from, block: false, cls: "cm-review-line cm-review-high" },
    { from: state.doc.line(2).to, block: true, cls: undefined },
  ]);
});
test("replacing the notes removes the old decorations and maps through edits", () => {
  let state = EditorState.create({ doc: "a\nb\nc", extensions: [notesField] });
  state = state.update({ effects: setNotes.of(placeNotes([n(2)], 3)) }).state;
  state = state.update({ changes: { from: 0, insert: "new line\n" } }).state;
  expect(state.field(notesField).size).toBe(2);
  state = state.update({ effects: setNotes.of([]) }).state;
  expect(state.field(notesField).size).toBe(0);
});
