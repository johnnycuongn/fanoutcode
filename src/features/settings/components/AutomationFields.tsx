// The project's automation: tests, advancing on green, whether Fanout Code may commit (default off, §6.6), the agent.
import type { ReactElement } from "react";
import type { ProjectId } from "@/lib/bindings";
import { useDetectedTest } from "@/lib/detectedTest";
import { projectScope, useSetting } from "../store";
import { InputSetting, SegmentedSetting, SwitchSetting } from "./fields";

export function AutomationFields({ projectId }: { projectId: ProjectId }): ReactElement {
  const scope = projectScope(projectId);
  const testCommand = useSetting(scope, "test_command");
  const detected = useDetectedTest(projectId);
  const set = Boolean(testCommand && testCommand.trim());
  const hasTests = set || detected !== null;
  const testHint = set
    ? undefined
    : detected
      ? `Found in ${detected.source}. Type a command to use another.`
      : "None found, so tasks skip the Test stage.";
  return (
    <>
      <SwitchSetting scope={scope} settingKey="test_enabled" label="Tests enabled" />
      <InputSetting
        scope={scope}
        settingKey="test_command"
        label="Test command"
        mono
        placeholder={detected?.command ?? "None"}
        {...(testHint ? { hint: testHint } : {})}
      />
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
