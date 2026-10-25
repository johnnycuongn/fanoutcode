// Display rules for "Checked in code". Verdicts are signals, not proof. Copied from LiveSpec (src/lib/checks.ts);
// Plan 05's `LiveSpecCheck.verdict` is livespec-core's serialised string, so an unknown value reads as Unclear.
import type { LiveSpecCheck } from "@/lib/bindings";

export type Verdict = "confirmed" | "contradicted" | "unclear" | "notShown" | "notFound";
export type VerdictTone = "accent" | "bad" | "mute";

export const VERDICT: Record<Verdict, { mark: string; label: string; tone: VerdictTone }> = {
  confirmed: { mark: "✓", label: "Confirmed", tone: "accent" },
  contradicted: { mark: "✗", label: "Contradicted", tone: "bad" },
  unclear: { mark: "?", label: "Unclear", tone: "mute" },
  notShown: { mark: "–", label: "Not shown in this code", tone: "mute" },
  notFound: { mark: "–", label: "Not found in indexed code", tone: "mute" },
};

export const verdictOf = (verdict: string): (typeof VERDICT)[Verdict] =>
  VERDICT[(verdict in VERDICT ? verdict : "unclear") as Verdict];

/** `done` verdicts so far; `stopped` once the rest were dropped. */
export function checkSummary(checking: { count: number; total: number }, done: number, stopped: boolean): string {
  const conflicts = checking.total === 1 ? "conflict" : "conflicts";
  if (!stopped && done < checking.count) return `Checking ${checking.count} of ${checking.total} ${conflicts}…`;
  return `${done} of ${checking.total} ${conflicts} checked`;
}

export const checkLocation = (r: LiveSpecCheck) => (r.path ? `${r.path}${r.line ? `:${r.line}` : ""}` : null);

/** One-line display text: inline `code` and **bold** markers removed. */
export const plainClaim = (claim: string) => claim.replace(/\*\*|`/g, "");
