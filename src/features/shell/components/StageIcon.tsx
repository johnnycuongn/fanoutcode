// The icon of a task stage, in the stage's colour, for a view tab. "many" marks a view whose tasks sit in more than
// one stage. Decorative: the tab carries the stage in its tooltip.
import {
  CircleCheck,
  FileText,
  FlaskConical,
  GitCommitHorizontal,
  Hammer,
  Layers,
  ListChecks,
  ScanSearch,
  type LucideIcon,
} from "lucide-react";
import type { ReactElement } from "react";
import { cx, stageClasses, type StageToken } from "@/components/ui";
import type { Stage } from "@/lib/bindings";

export type StageIconKind = Stage | "many";

const ICONS: Record<StageIconKind, LucideIcon> = {
  Spec: FileText,
  Plan: ListChecks,
  Build: Hammer,
  Review: ScanSearch,
  Test: FlaskConical,
  Commit: GitCommitHorizontal,
  Done: CircleCheck,
  many: Layers,
};

export function StageIcon({ stage, className }: { stage: StageIconKind; className?: string }): ReactElement {
  const Icon = ICONS[stage];
  const colour = stage === "many" ? "text-accent" : stageClasses[stage.toLowerCase() as StageToken].text;
  return <Icon aria-hidden data-stage={stage.toLowerCase()} className={cx("size-3.5 shrink-0", colour, className)} />;
}
