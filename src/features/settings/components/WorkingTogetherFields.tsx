// How tasks sharing a repo work together (spec §8): whether the hook asks before an edit to another task's files,
// whether git commands that would commit or discard them are held, and how *Bring <base> in* integrates.
import type { ReactElement } from "react";
import type { ProjectId } from "@/lib/bindings";
import { projectScope } from "../store";
import { SegmentedSetting, SwitchSetting } from "./fields";

export function WorkingTogetherFields({ projectId }: { projectId: ProjectId }): ReactElement {
  const scope = projectScope(projectId);
  return (
    <>
      <SwitchSetting
        scope={scope}
        settingKey="ask_before_other_tasks_files"
        label="Ask before editing another task's files"
        hint="Off: the overlap is logged and the edit goes ahead."
      />
      <SwitchSetting
        scope={scope}
        settingKey="guard_git_for_other_tasks"
        label="Guard git against other tasks' uncommitted work"
        hint="Off: the command is logged and runs."
      />
      <SegmentedSetting
        scope={scope}
        settingKey="worktree_integration"
        label="Integration"
        options={[
          { value: "Merge", label: "Merge" },
          { value: "Rebase", label: "Rebase" },
        ]}
      />
    </>
  );
}
