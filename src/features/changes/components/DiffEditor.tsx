// The diff editor for one file (§6.4): CodeMirror 6 unified or side by side, the working side editable and saved with
// ⌘S, review notes pinned to their lines as React portals, and a bar when the agent changed the file underneath an
// unsaved edit (Review Focus 2).
import { useEffect, useState, useSyncExternalStore, type ReactElement } from "react";
import { createPortal } from "react-dom";
import { Button, EmptyState } from "@/components/ui";
import type { FileDiff } from "@/lib/bindings";
import type { Extension } from "@codemirror/state";
import { noteSlots, type PlacedNote } from "../decorations";
import { languageFor } from "../languages";
import type { DiffMode } from "../store";
import type { ReviewNote } from "../types";
import { NoteCard } from "./NoteCard";
import { useMergeEditor } from "./useMergeEditor";

export interface DiffEditorProps {
  path: string;
  diff: FileDiff;
  mode: DiffMode;
  buffer: string;
  notes: PlacedNote[];
  staleOnDisk: boolean;
  onChange(text: string): void;
  onSave(): void;
  onReload(): void;
  onReply?(note: ReviewNote, text: string): Promise<void>;
}

function useLanguage(path: string): Extension | null {
  const [lang, setLang] = useState<{ path: string; ext: Extension | null } | null>(null);
  useEffect(() => {
    let live = true;
    void languageFor(path).then((ext) => live && setLang({ path, ext }));
    return () => {
      live = false;
    };
  }, [path]);
  return lang?.path === path ? lang.ext : null;
}

function Editor(p: DiffEditorProps): ReactElement {
  const language = useLanguage(p.path);
  const host = useMergeEditor({ ...p, language });
  const slots = useSyncExternalStore(noteSlots.subscribe, noteSlots.get, noteSlots.get);
  return (
    <>
      <div ref={host} className="h-full min-h-0 overflow-auto font-mono text-[12px]" />
      {p.notes.map((n) => {
        const el = slots.get(n.note.id);
        return el
          ? createPortal(
              <NoteCard
                note={n.note}
                clamped={n.clamped}
                onReply={(text) => p.onReply?.(n.note, text) ?? Promise.resolve()}
              />,
              el,
              n.note.id,
            )
          : null;
      })}
    </>
  );
}

export function DiffEditor(p: DiffEditorProps): ReactElement {
  if (p.diff.binary) return <EmptyState size="sm" title="Binary file" />;
  return (
    <div className="flex h-full min-h-0 flex-col">
      {p.staleOnDisk && (
        <div role="alert" className="flex shrink-0 items-center gap-2 border-b border-line bg-well px-3 py-1 text-xs">
          <span className="flex-1 text-warn">File changed on disk since you opened it</span>
          <Button size="sm" variant="secondary" onClick={p.onReload}>
            Reload
          </Button>
          <Button size="sm" variant="danger" onClick={p.onSave}>
            Overwrite
          </Button>
        </div>
      )}
      <div className="min-h-0 flex-1">
        <Editor {...p} />
      </div>
    </div>
  );
}
