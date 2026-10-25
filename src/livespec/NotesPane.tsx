// The LiveSpec notes beside the spec. Copied from LiveSpec (src/components/NotesPane.tsx): the history browser, the
// sections picker and the brief prompt are gone, and the sections follow the task's LiveSpec level (Off, Simple,
// Heavy) instead of a per-tab choice.
import type { LiveSpecCheck, LiveSpecLevel } from "@/lib/bindings";
import { Button, cx } from "@/components/ui";
import type { AnalysisState } from "./analysisState";
import { checkLocation, checkSummary, plainClaim, verdictOf } from "./checks";
import { MiniMarkdown } from "./MiniMarkdown";
import { SECTIONS, parseSections, sectionsForLevel, sectionsSentence } from "./sections";

interface Props {
  analysis: AnalysisState;
  stale: boolean;
  level: LiveSpecLevel;
  onRerun: () => void;
}

type RuleState = "idle" | "thinking" | "stale" | "fresh";

const RULE: Record<RuleState, string> = {
  idle: "bg-accent scale-y-0",
  fresh: "bg-accent",
  thinking: "bg-accent animate-fill motion-reduce:animate-none motion-reduce:opacity-50",
  // Dashed: the notes are still there, but the draft has moved on.
  stale: "bg-[repeating-linear-gradient(to_bottom,var(--color-accent)_0_6px,transparent_6px_12px)] opacity-70",
};

const OFF_HINT = "LiveSpec is off. Turn on Simple or Heavy to get notes as you write.";

function ruleState(a: AnalysisState, stale: boolean): RuleState {
  if (a.status === "thinking") return "thinking";
  if (stale) return "stale";
  return a.status === "done" ? "fresh" : "idle";
}

function statusText(a: AnalysisState, stale: boolean): string {
  if (a.status === "thinking") return "Thinking";
  if (a.status === "error") return "Couldn't think this through";
  if (stale) return "Your draft moved on. Pause to refresh.";
  if (a.status === "done") return "Up to date with your draft";
  return "Reading along";
}

/** The accent rule between the panes: solid when current, animated while thinking. */
function Rule({ state }: { state: RuleState }) {
  return (
    <div className="absolute inset-y-0 left-0 w-0.5 bg-line" aria-hidden>
      <div className={cx("absolute inset-0 origin-top", RULE[state])} />
    </div>
  );
}

/** Small uppercase heading for a note section. */
function SectionHeading({ children, dot }: { children: string; dot?: string }) {
  return (
    <span className="flex items-center gap-2 text-[11px] font-semibold tracking-wider text-mute uppercase">
      {dot && <span className={cx("size-1.5 rounded-full", dot)} aria-hidden />}
      {children}
    </span>
  );
}

const TONE = { accent: "text-accent", bad: "text-bad", mute: "text-mute" } as const;

function CheckRow({ check }: { check: LiveSpecCheck }) {
  const v = verdictOf(check.verdict);
  const where = checkLocation(check);
  return (
    <li className="grid grid-cols-[1rem_minmax(0,1fr)] gap-x-2 text-[13px] leading-snug">
      <span className={cx("font-semibold", TONE[v.tone])} aria-hidden>
        {v.mark}
      </span>
      <span className="min-w-0">
        <span className="block truncate" title={plainClaim(check.claim)}>
          {plainClaim(check.claim)}
        </span>
        <span className="text-xs text-mute">
          {v.label}
          {where && (
            <>
              {" · "}
              <code className="font-mono text-[0.85em]">{where}</code>
            </>
          )}
        </span>
      </span>
    </li>
  );
}

export function NotesPane({ analysis: a, stale, level, onRerun }: Props) {
  const sections = parseSections(a.markdown);
  const shown = a.status !== "idle" && a.status !== "error";

  return (
    <aside className="relative flex min-h-0 flex-col bg-well" aria-label="Notes">
      <Rule state={ruleState(a, stale)} />
      <div className="min-h-0 flex-1 overflow-y-auto px-7 pt-7 pb-10">
        {level === "Off" && <p className="mb-4 text-[13px] text-mute">{OFF_HINT}</p>}
        {(level !== "Off" || shown) && (
          <p className="mb-5 text-xs text-mute" aria-live="polite">
            {statusText(a, stale)}
          </p>
        )}
        {a.notices.map((n, i) => (
          <p key={i} className="mb-2 text-xs text-mute">
            {n}
          </p>
        ))}
        {a.partial && a.status === "done" && (
          <p className="mb-4 text-xs text-mute">
            Some sections are missing.{" "}
            <Button variant="link" size="sm" onClick={onRerun}>
              Try again
            </Button>
          </p>
        )}

        {a.status === "error" && (
          <div className="grid justify-items-start gap-2 text-[13px] text-bad" role="alert">
            <p className="whitespace-pre-wrap">{a.error}</p>
            <Button variant="secondary" size="sm" onClick={onRerun}>
              Try again
            </Button>
          </div>
        )}

        {a.status === "idle" && level !== "Off" && (
          <div className="max-w-[46ch] space-y-2.5 text-[13px] leading-relaxed text-mute">
            <p>Write your spec on the left. When you pause, notes appear here:</p>
            <p>{sectionsSentence(sectionsForLevel(level))}</p>
          </div>
        )}

        {shown && (
          <div className={cx("grid gap-6 transition-opacity motion-reduce:transition-none", stale && "opacity-60")}>
            {SECTIONS.map((s) =>
              sections[s.key] !== undefined ? (
                <section key={s.key} className="grid gap-1.5">
                  <h2>
                    <SectionHeading dot={s.dot}>{s.title}</SectionHeading>
                  </h2>
                  <MiniMarkdown text={sections[s.key]!} />
                </section>
              ) : null,
            )}
            {a.status === "thinking" && !a.markdown && (
              <p className="text-[13px] text-mute">Reading your spec against the project…</p>
            )}
            {a.checking && (
              <section className="grid gap-1.5">
                <h2>
                  <SectionHeading dot="bg-ink">Checked in code</SectionHeading>
                </h2>
                <p className="text-xs text-mute">{checkSummary(a.checking, a.checks.length, a.checksStopped)}</p>
                <ul className="grid gap-2">
                  {a.checks.map((c, i) => (
                    <CheckRow key={i} check={c} />
                  ))}
                </ul>
              </section>
            )}
          </div>
        )}
      </div>
    </aside>
  );
}
