// First run in three steps: what the app does and which agent CLIs this Mac has, where each task works (its own
// worktree, recommended), then the theme (previewed live). The last two are recorded together.
import { useState, type ReactElement } from "react";
import { Button, EmptyState, Panel, Segmented } from "@/components/ui";
import { commands, type ThemePreference, type WorktreeConvention } from "@/lib/bindings";
import { applyTheme } from "@/lib/theme";
import { AgentCheck } from "./AgentCheck";
import { ConventionChoice } from "./ConventionChoice";

const THEME_OPTIONS = (["System", "Light", "Dark"] as const).map((t) => ({ value: t, label: t }));
const STEPS = 3;

export function Onboarding({ onDone }: { onDone(): void }): ReactElement {
  const [step, setStep] = useState(1);
  const [convention, setConvention] = useState<WorktreeConvention>("FanoutCodeManaged");
  const [theme, setTheme] = useState<ThemePreference>("System");
  const [error, setError] = useState<string | null>(null);
  const pickTheme = (t: ThemePreference) => {
    setTheme(t);
    applyTheme(t);
  };
  const finish = async () => {
    const r = await commands.settingsFirstRunComplete(convention, theme);
    if (r?.status === "error") setError(r.error);
    else onDone();
  };
  return (
    <main className="grid h-full place-items-center bg-page p-6 text-ink">
      <Panel variant="card" padding="md" className="flex w-full max-w-2xl flex-col gap-4">
        <EmptyState size="sm" title="Welcome to Fanout Code" description={`Step ${step} of ${STEPS}`} />
        {step === 1 && (
          <>
            <p className="text-sm text-ink">
              Fanout Code runs AI coding agents from a board. Each task gets its own agent in a terminal and moves
              through Spec, Plan, Build, Review, Test and Commit. You read and approve every step.
            </p>
            <AgentCheck />
          </>
        )}
        {step === 2 && (
          <>
            <h2 className="font-medium">Where should each task work?</h2>
            <ConventionChoice value={convention} onChange={setConvention} />
          </>
        )}
        {step === 3 && (
          <>
            <h2 className="font-medium">Theme</h2>
            <Segmented aria-label="Theme" options={THEME_OPTIONS} value={theme} onValueChange={pickTheme} />
          </>
        )}
        {error && <p className="text-sm text-bad">{error}</p>}
        <div className="flex justify-end gap-2">
          {step > 1 && (
            <Button variant="ghost" onClick={() => setStep(step - 1)}>
              Back
            </Button>
          )}
          <Button variant="primary" onClick={() => (step < STEPS ? setStep(step + 1) : void finish())}>
            {step < STEPS ? "Continue" : "Start"}
          </Button>
        </div>
      </Panel>
    </main>
  );
}
