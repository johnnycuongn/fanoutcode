// The chip on a card. The board defines no hand-back copy of its own: the text comes from `chipText` in
// `@/lib/format` (overview A2, the TypeScript mirror of `handback::chip_text`).
import type { Task } from "@/lib/bindings";
import { chipText } from "@/lib/format";
import type { CardChipModel } from "./types";

/** The first non-empty line of `text`, trimmed, cut to `max` characters with an ellipsis. */
export function firstLine(text: string, max = 80): string {
  const line =
    text
      .split(/\r?\n/)
      .map((l) => l.trim())
      .find((l) => l.length > 0) ?? "";
  return line.length > max ? `${line.slice(0, max - 1)}…` : line;
}

export function chipFor(task: Pick<Task, "stage" | "state">): CardChipModel {
  const { state, stage } = task;
  switch (state.type) {
    case "Working":
      return { tone: "working", text: "Working" };
    case "Idle":
      return { tone: "idle", text: "Idle" };
    case "NeedsYou": {
      const { reason } = state;
      const text = firstLine(chipText(reason, stage));
      if (reason.type === "TestFailed") return { tone: "needs_you", text, detail: `exit ${reason.exit_code}` };
      if (reason.type === "Error") return { tone: "needs_you", text, detail: firstLine(reason.message) };
      const done = reason.type === "StageDone" && (stage === "Test" || stage === "Done");
      return { tone: done ? "ok" : "needs_you", text };
    }
  }
}
