// First run: pick the worktree lifecycle, then the theme (previewed live), then record both.
import { useState, type ReactElement } from "react";
import { Button, EmptyState, Panel, Segmented } from "@/components/ui";
import { commands, type ThemePreference, type WorktreeConvention } from "@/lib/bindings";
import { applyTheme } from "@/lib/theme";
import { ConventionChoice } from "./ConventionChoice";

const THEME_OPTIONS = (["System", "Light", "Dark"] as const).map((t) => ({ value: t, label: t }));

export function Onboarding({ onDone }: { onDone(): void }): ReactElement {
  const [step, setStep] = useState<1 | 2>(1);
  const [convention, setConvention] = useState<WorktreeConvention>("ClaudeCode");
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
        <EmptyState size="sm" title="Welcome to Team Code" description={step === 1 ? "Step 1 of 2" : "Step 2 of 2"} />
        {step === 1 ? (
          <>
            <h2 className="font-medium">How should task worktrees be managed?</h2>
            <ConventionChoice value={convention} onChange={setConvention} />
          </>
        ) : (
          <>
            <h2 className="font-medium">Theme</h2>
            <Segmented aria-label="Theme" options={THEME_OPTIONS} value={theme} onValueChange={pickTheme} />
          </>
        )}
        {error && <p className="text-sm text-bad">{error}</p>}
        <div className="flex justify-end gap-2">
          {step === 2 && (
            <Button variant="ghost" onClick={() => setStep(1)}>
              Back
            </Button>
          )}
          <Button onClick={() => (step === 1 ? setStep(2) : void finish())}>Continue</Button>
        </div>
      </Panel>
    </main>
  );
}
