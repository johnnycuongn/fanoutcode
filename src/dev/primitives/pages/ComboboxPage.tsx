import { useState, type ReactElement } from "react";
import { Badge, Combobox, Field, type ComboboxOption } from "../../../components/ui";

const roles: ComboboxOption[] = [
  { value: "frontend", label: "frontend", hint: "UI and styling" },
  { value: "backend", label: "backend", hint: "APIs and data" },
  { value: "best-practice", label: "best-practice reviewer", hint: "review" },
  { value: "consistency", label: "codebase-consistency reviewer", hint: "review" },
  { value: "security", label: "security reviewer", hint: "review" },
];

const many: ComboboxOption[] = Array.from({ length: 60 }, (_, i) => ({
  value: `project-${i + 1}`,
  label: `Project ${i + 1}`,
}));

export default function ComboboxPage(): ReactElement {
  const [role, setRole] = useState<string | null>("backend");
  const [project, setProject] = useState<string | null>(null);
  return (
    <div className="flex max-w-xs flex-col gap-4">
      <h1 className="text-lg font-semibold">Combobox</h1>
      <Combobox options={roles} value={role} onValueChange={setRole} clearable aria-label="Role" />
      <Combobox
        options={many}
        value={project}
        onValueChange={setProject}
        placeholder="Pick a project"
        aria-label="Project"
      />
      <Combobox
        options={roles}
        value="removed"
        onValueChange={() => {}}
        placeholder="Value not in the list"
        aria-label="Missing"
      />
      <Combobox options={roles} value="frontend" onValueChange={() => {}} disabled aria-label="Disabled" />
      <Field label="Agent role" error="Pick a role.">
        <Combobox options={roles} value={null} onValueChange={() => {}} />
      </Field>
      <Combobox
        options={roles}
        value={role}
        onValueChange={setRole}
        aria-label="Custom option"
        renderOption={(option, selected) => (
          <>
            <span>{option.label}</span>
            <Badge size="sm" tone={selected ? "accent" : "neutral"} className="ml-auto">
              {option.hint}
            </Badge>
          </>
        )}
      />
    </div>
  );
}
