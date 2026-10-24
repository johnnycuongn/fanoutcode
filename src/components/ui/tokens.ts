// Shared unions and the literal class maps Tailwind needs to see. Colour-by-prop goes through these maps, never
// through template strings.
import type { Swatch } from "@/lib/theme";

export type { Swatch };
export type Size = "sm" | "md" | "lg";
export type Tone = "neutral" | "accent" | "ok" | "warn" | "bad";
export type StageToken = "spec" | "plan" | "build" | "review" | "test" | "commit" | "done";
export type AgentStatus = "working" | "needs-you" | "idle";

export const STAGES: readonly StageToken[] = ["spec", "plan", "build", "review", "test", "commit", "done"];

export type FillClasses = { soft: string; solid: string; text: string };

export const toneClasses: Record<Tone, FillClasses> = {
  neutral: { soft: "bg-well text-mute", solid: "bg-mute text-accent-ink", text: "text-mute" },
  accent: { soft: "bg-accent/15 text-accent", solid: "bg-accent text-accent-ink", text: "text-accent" },
  ok: { soft: "bg-ok/15 text-ok", solid: "bg-ok text-accent-ink", text: "text-ok" },
  warn: { soft: "bg-warn/15 text-warn", solid: "bg-warn text-accent-ink", text: "text-warn" },
  bad: { soft: "bg-bad/15 text-bad", solid: "bg-bad text-accent-ink", text: "text-bad" },
};

export const stageClasses: Record<StageToken, FillClasses> = {
  spec: { soft: "bg-stage-spec/15 text-stage-spec", solid: "bg-stage-spec text-accent-ink", text: "text-stage-spec" },
  plan: { soft: "bg-stage-plan/15 text-stage-plan", solid: "bg-stage-plan text-accent-ink", text: "text-stage-plan" },
  build: {
    soft: "bg-stage-build/15 text-stage-build",
    solid: "bg-stage-build text-accent-ink",
    text: "text-stage-build",
  },
  review: {
    soft: "bg-stage-review/15 text-stage-review",
    solid: "bg-stage-review text-accent-ink",
    text: "text-stage-review",
  },
  test: { soft: "bg-stage-test/15 text-stage-test", solid: "bg-stage-test text-accent-ink", text: "text-stage-test" },
  commit: {
    soft: "bg-stage-commit/15 text-stage-commit",
    solid: "bg-stage-commit text-accent-ink",
    text: "text-stage-commit",
  },
  done: { soft: "bg-stage-done/15 text-stage-done", solid: "bg-stage-done text-accent-ink", text: "text-stage-done" },
};

// Same ten names as `SWATCHES` in src/lib/theme.ts (the shared task and project palette).
export const swatchClasses: Record<Swatch, FillClasses> = {
  rose: {
    soft: "bg-swatch-rose/15 text-swatch-rose",
    solid: "bg-swatch-rose text-accent-ink",
    text: "text-swatch-rose",
  },
  orange: {
    soft: "bg-swatch-orange/15 text-swatch-orange",
    solid: "bg-swatch-orange text-accent-ink",
    text: "text-swatch-orange",
  },
  yellow: {
    soft: "bg-swatch-yellow/15 text-swatch-yellow",
    solid: "bg-swatch-yellow text-accent-ink",
    text: "text-swatch-yellow",
  },
  lime: {
    soft: "bg-swatch-lime/15 text-swatch-lime",
    solid: "bg-swatch-lime text-accent-ink",
    text: "text-swatch-lime",
  },
  emerald: {
    soft: "bg-swatch-emerald/15 text-swatch-emerald",
    solid: "bg-swatch-emerald text-accent-ink",
    text: "text-swatch-emerald",
  },
  cyan: {
    soft: "bg-swatch-cyan/15 text-swatch-cyan",
    solid: "bg-swatch-cyan text-accent-ink",
    text: "text-swatch-cyan",
  },
  indigo: {
    soft: "bg-swatch-indigo/15 text-swatch-indigo",
    solid: "bg-swatch-indigo text-accent-ink",
    text: "text-swatch-indigo",
  },
  violet: {
    soft: "bg-swatch-violet/15 text-swatch-violet",
    solid: "bg-swatch-violet text-accent-ink",
    text: "text-swatch-violet",
  },
  fuchsia: {
    soft: "bg-swatch-fuchsia/15 text-swatch-fuchsia",
    solid: "bg-swatch-fuchsia text-accent-ink",
    text: "text-swatch-fuchsia",
  },
  slate: {
    soft: "bg-swatch-slate/15 text-swatch-slate",
    solid: "bg-swatch-slate text-accent-ink",
    text: "text-swatch-slate",
  },
};

export function statusTone(status: AgentStatus): Tone {
  return status === "working" ? "ok" : status === "needs-you" ? "warn" : "neutral";
}

export function statusLabel(status: AgentStatus): string {
  return status === "working" ? "Working" : status === "needs-you" ? "Needs you" : "Idle";
}

/** Picks the fill for a colour-by-prop component; precedence swatch > stage > tone, default neutral. */
export function pickFill(opts: { swatch?: Swatch; stage?: StageToken; tone?: Tone }): FillClasses {
  if (opts.swatch) return swatchClasses[opts.swatch];
  if (opts.stage) return stageClasses[opts.stage];
  return toneClasses[opts.tone ?? "neutral"];
}
