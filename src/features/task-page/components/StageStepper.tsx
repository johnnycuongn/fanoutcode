// Six stage chips (Done marks all six done); the current one breathes in its stage colour. Test reads "Test
// skipped" when the project has no test command (set or detected) or tests are off, instead of a green tick.
import type { ReactElement } from "react";
import { Chip } from "@/components/ui";
import type { Stage } from "@/lib/bindings";
import { stageToken } from "@/lib/format";

export const STEPPER_STAGES: Stage[] = ["Spec", "Plan", "Build", "Review", "Test", "Commit"];

export function StageStepper({ stage, skipTest = false }: { stage: Stage; skipTest?: boolean }): ReactElement {
  const at = stage === "Done" ? STEPPER_STAGES.length : STEPPER_STAGES.indexOf(stage);
  return (
    <ol className="flex items-center gap-1" aria-label="Stages">
      {STEPPER_STAGES.map((s, i) => {
        const state = i < at ? "done" : i === at ? "current" : "upcoming";
        if (s === "Test" && skipTest && state !== "current") {
          return (
            <li key={s} className="flex">
              <Chip
                size="sm"
                data-state="skipped"
                title="No test command, so this task skips Test. Set one in Settings, Automation."
                className="bg-transparent text-mute line-through"
              >
                Test skipped
              </Chip>
            </li>
          );
        }
        return (
          <li key={s} className="flex">
            {state === "done" && (
              <Chip size="sm" tone="ok" data-state="done">
                {s}
              </Chip>
            )}
            {state === "current" && (
              <Chip
                size="sm"
                stage={stageToken(s)}
                data-state="current"
                aria-current="step"
                className="motion-safe:animate-breathe"
              >
                {s}
              </Chip>
            )}
            {state === "upcoming" && (
              <Chip size="sm" data-state="upcoming" className="bg-transparent text-mute">
                {s}
              </Chip>
            )}
          </li>
        );
      })}
    </ol>
  );
}
