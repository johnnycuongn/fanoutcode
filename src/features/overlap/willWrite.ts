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

/** True for a `Write`, whose new content is diffed against the current file. */
export function needsCurrentFile(p: PendingDecision | null): boolean {
  return p?.type === "Edit" && p.tool === "Write";
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
