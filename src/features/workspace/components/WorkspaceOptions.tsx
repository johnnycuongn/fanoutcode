// Workspace options: the project layer and the board mode, written through the workspace file.
import type { ReactElement } from "react";
import { Field, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui";
import { commands, type ProjectLayer, type WorkspaceBoard } from "@/lib/bindings";
import { loadSettings, useSetting, workspaceScope } from "@/features/settings";
import { workspaceStore } from "../store";
import { Section } from "./Section";

const LAYERS: Array<{ value: ProjectLayer; label: string }> = [
  { value: "MultiRoot", label: "Multi-root sidebar" },
  { value: "Rail", label: "Project rail" },
];
const BOARDS: Array<{ value: WorkspaceBoard; label: string }> = [
  { value: "Swimlanes", label: "Swimlanes" },
  { value: "OneProject", label: "One project at a time" },
  { value: "Mixed", label: "Mixed" },
];

function Choice<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T | undefined;
  options: Array<{ value: T; label: string }>;
  onChange(v: T): void;
}): ReactElement {
  return (
    <Field label={label} layout="row">
      <Select value={value} onValueChange={(v) => onChange(v as T)}>
        <SelectTrigger size="sm" aria-label={label}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {options.map((o) => (
            <SelectItem key={o.value} value={o.value}>
              {o.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Field>
  );
}

export function WorkspaceOptions({ workspaceId }: { workspaceId: string }): ReactElement {
  const scope = workspaceScope(workspaceId);
  const layer = useSetting(scope, "workspace_project_layer");
  const board = useSetting(scope, "workspace_board");
  const set = async (key: "workspace_project_layer" | "workspace_board", value: string) => {
    const r = await commands.workspacesSetSetting(workspaceId, key, value);
    if (r.status === "ok") {
      workspaceStore.set({ workspace: r.data });
      await loadSettings(scope);
    } else workspaceStore.set({ error: r.error });
  };
  return (
    <Section title="Options">
      <Choice
        label="Project layer"
        value={layer}
        options={LAYERS}
        onChange={(v) => void set("workspace_project_layer", v)}
      />
      <Choice label="Board" value={board} options={BOARDS} onChange={(v) => void set("workspace_board", v)} />
    </Section>
  );
}
