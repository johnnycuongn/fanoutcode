// Builds and keeps the CodeMirror editor for one file: a unified view (the working side with the base folded in as
// deletions) or a side-by-side MergeView (base read-only on the left). The editor is rebuilt when the diff or the
// mode changes; notes and a reloaded buffer are pushed into the live editor.
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { MergeView, unifiedMergeView } from "@codemirror/merge";
import { Annotation, EditorState, type Extension } from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { useEffect, useRef } from "react";
import type { FileDiff } from "@/lib/bindings";
import { notesField, reviewTheme, setNotes, type PlacedNote } from "../decorations";
import type { DiffMode } from "../store";

const external = Annotation.define<boolean>();

export interface MergeEditorOpts {
  path: string;
  diff: FileDiff;
  mode: DiffMode;
  buffer: string;
  notes: PlacedNote[];
  language: Extension | null;
  onChange(text: string): void;
  onSave(): void;
}

export function useMergeEditor(o: MergeEditorOpts) {
  const host = useRef<HTMLDivElement | null>(null);
  const view = useRef<EditorView | null>(null);
  const handlers = useRef({ onChange: o.onChange, onSave: o.onSave });
  const deleted = o.diff.status === "Deleted";

  useEffect(() => {
    handlers.current = { onChange: o.onChange, onSave: o.onSave };
  });

  useEffect(() => {
    const el = host.current;
    if (!el) return;
    const base: Extension[] = [
      lineNumbers(),
      EditorView.lineWrapping,
      reviewTheme,
      ...(o.language ? [o.language] : []),
    ];
    const working: Extension[] = [
      ...base,
      history(),
      notesField,
      keymap.of([{ key: "Mod-s", run: () => (handlers.current.onSave(), true) }, ...defaultKeymap, ...historyKeymap]),
      EditorState.readOnly.of(deleted),
      EditorView.updateListener.of((u) => {
        if (u.docChanged && !u.transactions.some((t) => t.annotation(external)))
          handlers.current.onChange(u.state.doc.toString());
      }),
    ];
    let destroy: () => void;
    if (o.mode === "split") {
      const mv = new MergeView({
        a: { doc: o.diff.before, extensions: [...base, EditorState.readOnly.of(true), EditorView.editable.of(false)] },
        b: { doc: deleted ? o.diff.before : o.buffer, extensions: working },
        parent: el,
      });
      view.current = mv.b;
      destroy = () => mv.destroy();
    } else {
      const v = new EditorView({
        parent: el,
        state: EditorState.create({
          doc: deleted ? o.diff.before : o.buffer,
          extensions: [...working, ...(deleted ? [] : [unifiedMergeView({ original: o.diff.before })])],
        }),
      });
      view.current = v;
      destroy = () => v.destroy();
    }
    return () => {
      destroy();
      view.current = null;
    };
    // rebuilt only for a new diff (its file or base), mode or language; buffer and notes are pushed below. Keyed on the
    // diff itself, not the selected path: a new selection renders once with the previous file's diff before its own
    // arrives, and a rebuild then would keep that file's base on the left
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [o.diff.repo_root, o.diff.path, o.diff.before, o.mode, o.language, deleted]);

  useEffect(() => {
    const v = view.current;
    if (!v || deleted || v.state.doc.toString() === o.buffer) return;
    v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert: o.buffer }, annotations: external.of(true) });
  }, [o.buffer, deleted]);

  useEffect(() => {
    view.current?.dispatch({ effects: setNotes.of(o.notes) });
  }, [o.notes, o.path, o.mode, o.language]);

  return host;
}
