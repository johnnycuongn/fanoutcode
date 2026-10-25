// What a stopped session was about to do (§5.4), from the pending decision's tool input: the two texts to diff for
// an edit, or the command for a shell command. `null` while the tool input has not landed yet.
import type { JsonValue, PendingDecision } from "@/lib/bindings";

export type WillWrite =
  | { type: "Diff"; before: string; after: string }
  | { type: "Command"; command: string }
  | { type: "Raw"; text: string };

type Obj = Partial<{ [key in string]: JsonValue }>;

function obj(v: JsonValue | undefined): Obj | null {
  return v !== null && typeof v === "object" && !Array.isArray(v) ? v : null;
}

function str(v: JsonValue | undefined): string {
  return typeof v === "string" ? v : "";
}

/** True for a `Write`, whose new content is diffed against the current file, and an `Edit`, whose old string is
 * placed in it ("lines a–b"). */
export function needsCurrentFile(p: PendingDecision | null): boolean {
  return p?.type === "Edit" && (p.tool === "Write" || p.tool === "Edit");
}

/** Where an `Edit`'s old string falls in the current file, as "lines a–b" (1-based, "line a" for one line); `null`
 * for anything else, or when the file does not hold the old string. */
export function editLines(p: PendingDecision | null, currentFile: string | null): string | null {
  if (p?.type !== "Edit" || p.tool !== "Edit" || currentFile === null) return null;
  const old = str(obj(p.input)?.old_string);
  const at = old === "" ? -1 : currentFile.indexOf(old);
  if (at < 0) return null;
  const lineOf = (i: number) => currentFile.slice(0, i).split("\n").length;
  const from = lineOf(at);
  const to = lineOf(at + old.replace(/\n$/, "").length);
  return from === to ? `line ${from}` : `lines ${from}–${to}`;
}

export function willWrite(p: PendingDecision | null, currentFile: string | null): WillWrite | null {
  if (!p) return null;
  if (p.type === "Command") return { type: "Command", command: p.command };
  const input = obj(p.input);
  if (!input) return null;
  if (p.tool === "Write") {
    if (currentFile === null) return null;
    return { type: "Diff", before: currentFile, after: str(input.content) };
  }
  if (p.tool === "Edit") return { type: "Diff", before: str(input.old_string), after: str(input.new_string) };
  if (p.tool === "MultiEdit" && Array.isArray(input.edits)) {
    const edits = input.edits.map(obj).filter((e): e is Obj => e !== null);
    return {
      type: "Diff",
      before: edits.map((e) => str(e.old_string)).join("\n"),
      after: edits.map((e) => str(e.new_string)).join("\n"),
    };
  }
  return { type: "Raw", text: JSON.stringify(p.input, null, 2) };
}
