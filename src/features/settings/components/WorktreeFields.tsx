// The project's worktree defaults: whether new tasks get one, which lifecycle, and where Fanout Code puts its own.
import type { ReactElement } from "react";
import { Switch } from "@/components/ui";
import type { ProjectId } from "@/lib/bindings";
import { projectScope, setSetting, useSetting } from "../store";
import { InputSetting, SegmentedSetting } from "./fields";

export function WorktreeFields({ projectId, root }: { projectId: ProjectId; root?: string }): ReactElement {
  const scope = projectScope(projectId);
  const worktree = useSetting(scope, "worktree");
  const convention = useSetting(scope, "worktree_convention");
  return (
    <>
      <Switch
        label="Use a worktree for new tasks"
        checked={worktree === "On"}
        onCheckedChange={(on) => void setSetting(scope, "worktree", on ? "On" : "Off")}
      />
      <SegmentedSetting
        scope={scope}
        settingKey="worktree_convention"
        label="Worktree lifecycle"
        inherit
        options={[
          { value: "ClaudeCode", label: "Claude Code's own" },
          { value: "FanoutCodeManaged", label: "Fanout Code-managed" },
        ]}
      />
      {convention === "FanoutCodeManaged" && (
        <InputSetting
          scope={scope}
          settingKey="worktrees_dir"
          label="Worktrees dir"
          mono
          placeholder={`${root ?? "<root>"}/.worktrees/`}
        />
      )}
    </>
  );
}
