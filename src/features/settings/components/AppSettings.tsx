// The App tab of Settings: every app-level key of Plan 03's registry (A12) that has a screen. Not here on purpose:
// `first_run_complete` (onboarding), `livespec_level` (Plan 08's Spec switch), `jira` and `jira_transitions`
// (Plan 10's Jira linking); the linking policy is `Workspace.policy` (Plan 10's workspace tab).
import type { ReactElement } from "react";
import { appScope, useSettings } from "../store";
import { AppearanceFields } from "./AppearanceFields";
import { InputSetting, SwitchSetting } from "./fields";
import { TeamViewFields } from "./TeamViewFields";

export function AppSettings(): ReactElement {
  const loaded = useSettings(appScope()) !== undefined;
  if (!loaded) return <p className="text-sm text-mute">Loading settings…</p>;
  const scope = appScope();
  return (
    <div className="flex flex-col gap-3">
      <AppearanceFields />
      <TeamViewFields scope={scope} />
      <SwitchSetting
        scope={scope}
        settingKey="cost_overview"
        label="Cost overview"
        hint="Shows cost and tokens wherever a task appears"
      />
      <InputSetting scope={scope} settingKey="scrollback_lines" label="Scrollback lines" type="number" min={1000} />
    </div>
  );
}
