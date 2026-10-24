// App settings: the team view and the workspace defaults.
import type { ReactElement } from "react";
import type { Scope } from "@/lib/bindings";
import { SegmentedSetting } from "./fields";

export function TeamViewFields({ scope }: { scope: Scope }): ReactElement {
  return (
    <>
      <SegmentedSetting
        scope={scope}
        settingKey="team_view_arrangement"
        label="Team view arrangement"
        options={[
          { value: "HubGraph", label: "Hub graph" },
          { value: "Flow", label: "Flow canvas" },
        ]}
      />
      <SegmentedSetting
        scope={scope}
        settingKey="team_view_placement"
        label="Team view placement"
        options={[
          { value: "DockGrows", label: "Dock grows" },
          { value: "Swap", label: "Swap with terminal" },
          { value: "OwnPane", label: "Own pane" },
        ]}
      />
      <SegmentedSetting
        scope={scope}
        settingKey="team_canvas_layout"
        label="Team canvas layout"
        options={[
          { value: "AutoReflow", label: "Auto re-flow" },
          { value: "Pinned", label: "Pinned" },
        ]}
      />
      <SegmentedSetting
        scope={scope}
        settingKey="team_drill_in_lens"
        label="Team drill-in lens"
        options={[
          { value: "ByFile", label: "By file" },
          { value: "EventStream", label: "Event stream" },
        ]}
      />
      <SegmentedSetting
        scope={scope}
        settingKey="workspace_project_layer"
        label="Workspace project layer"
        options={[
          { value: "MultiRoot", label: "Multi-root sidebar" },
          { value: "Rail", label: "Project rail" },
        ]}
      />
      <SegmentedSetting
        scope={scope}
        settingKey="workspace_board"
        label="Workspace board"
        options={[
          { value: "Swimlanes", label: "Swimlanes" },
          { value: "OneProject", label: "One project at a time" },
          { value: "Mixed", label: "Mixed" },
        ]}
      />
    </>
  );
}
