import type { ReactElement } from "react";
import { stageClasses, cx, type StageToken } from "@/components/ui";
import type { Stage } from "@/lib/bindings";
import { STAGES, stageIndex } from "../columns";

const STEPS = STAGES.slice(0, 6); // Spec … Commit; Done is the end, not a step

/** Six segments, Spec to Commit: done, current (in the stage colour) and upcoming. */
export function CardStepper({ stage }: { stage: Stage }): ReactElement {
  const at = stageIndex(stage);
  const position = Math.min(at + 1, STEPS.length);
  return (
    <ul aria-label={`Stage ${stage}, ${position} of ${STEPS.length}`} className="flex w-full gap-0.5">
      {STEPS.map((step, i) => {
        const state = at > i || stage === "Done" ? "done" : at === i ? "current" : "upcoming";
        return (
          <li
            key={step}
            data-state={state}
            title={step}
            className={cx(
              "h-1 flex-1 rounded-full",
              state === "done" && "bg-mute/50",
              state === "current" && cx("bg-current", stageClasses[step.toLowerCase() as StageToken].text),
              state === "upcoming" && "bg-line",
            )}
          />
        );
      })}
    </ul>
  );
}
