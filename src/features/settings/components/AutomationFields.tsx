// The project's automation: tests (a command found in the project's files is offered, never run unasked), advancing
// on green, whether Fanout Code may commit (default off, §6.6), the agent.
import type { ReactElement } from "react";
import type { ProjectId } from "@/lib/bindings";
import { useDetectedTest } from "@/lib/detectedTest";
import { Button, Field } from "@/components/ui";
import { projectScope, setSetting, useSetting } from "../store";
import { InputSetting, SegmentedSetting, SwitchSetting } from "./fields";

export function AutomationFields({ projectId }: { projectId: ProjectId }): ReactElement {
  const scope = projectScope(projectId);
  const testCommand = useSetting(scope, "test_command");
  const detected = useDetectedTest(projectId);
  const hasTests = Boolean(testCommand && testCommand.trim());
  return (
    <>
      <SwitchSetting scope={scope} settingKey="test_enabled" label="Tests enabled" />
      <InputSetting
        scope={scope}
        settingKey="test_command"
        label="Test command"
        mono
        placeholder="None"
        {...(hasTests ? {} : { hint: "Without one, tasks skip the Test stage." })}
      />
      {!hasTests && detected && (
        // a suggestion only: it runs the repository's own script, so it runs once the developer accepts it
        <Field label="Suggested" layout="row" hint={`From ${detected.source}. It runs this repository's own script.`}>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => void setSetting(scope, "test_command", detected.command)}
          >
            Use {detected.command}
          </Button>
        </Field>
      )}
      <SwitchSetting
        scope={scope}
        settingKey="advance_on_green"
        label="Advance on green"
        disabled={!hasTests}
        hint={hasTests ? "Moves the task to Commit when the tests pass." : "Set a test command first."}
      />
      <SwitchSetting
        scope={scope}
        settingKey="may_commit"
        label="Fanout Code may commit"
        hint="Off: the app drafts messages and you commit however you like."
      />
      <SegmentedSetting
        scope={scope}
        settingKey="agent_kind"
        label="Agent"
        options={[
          { value: "ClaudeCode", label: "Claude Code" },
          { value: "Codex", label: "Codex" },
          { value: "Shell", label: "Shell" },
        ]}
      />
    </>
  );
}
