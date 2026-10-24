import type { ReactElement, ReactNode } from "react";
import { Chip, StatusDot, type Tone } from "@/components/ui";
import type { CardChipModel, ChipTone } from "../types";

const TONE: Record<ChipTone, Tone> = { working: "ok", needs_you: "warn", ok: "ok", idle: "neutral" };

/** The state chip on a card (also used by the team canvas). `indicator` replaces the dot, e.g. with a ring. */
export function CardChip({ model, indicator }: { model: CardChipModel; indicator?: ReactNode }): ReactElement {
  const tone = TONE[model.tone];
  const text = model.detail ? `${model.text} · ${model.detail}` : model.text;
  return (
    <Chip
      size="sm"
      tone={tone}
      data-tone={model.tone}
      icon={indicator ?? <StatusDot data-testid="status-dot" tone={tone} size="sm" pulse={model.tone === "working"} />}
      className="min-w-0"
    >
      {text}
    </Chip>
  );
}
