// The Project tab of Settings (and the sidebar's Settings section): project name and colour, memory location,
// worktrees, automation, the detail-level override and, with the cost overview on, the project's costs.
import { useState, type ReactElement, type ReactNode } from "react";
import { Field, Input, Segmented, toast } from "@/components/ui";
import { commands, type ProjectId } from "@/lib/bindings";
import { projectsStore, upsertProject } from "@/lib/stores/projectsStore";
import { SWATCHES } from "@/lib/theme";
import { DETAIL_LEVELS, appScope, projectScope, useSetting } from "../store";
import { AutomationFields } from "./AutomationFields";
import { CostsPanel } from "./CostsPanel";
import { SegmentedSetting } from "./fields";
import { MemoryLocationField } from "./MemoryLocationField";
import { WorktreeFields } from "./WorktreeFields";

function Section({ title, children }: { title: string; children: ReactNode }): ReactElement {
  return (
    <section aria-label={title} className="flex flex-col gap-3">
      <h3 className="text-sm font-medium text-ink">{title}</h3>
      {children}
    </section>
  );
}

async function patchProject(id: ProjectId, patch: { name?: string; color?: string }) {
  const r = await commands.projectsUpdate(id, {
    name: patch.name ?? null,
    color: patch.color ?? null,
    memory_location: null,
  });
  if (r?.status === "ok") upsertProject(r.data);
  else if (r) toast.error(r.error);
}

function ProjectFields({ projectId }: { projectId: ProjectId }): ReactElement | null {
  const project = projectsStore.useSelector((s) => s.byId[projectId]);
  const [name, setName] = useState<string | null>(null);
  if (!project) return null;
  const commitName = () => {
    if (name !== null && name.trim() && name.trim() !== project.name)
      void patchProject(projectId, { name: name.trim() });
    setName(null);
  };
  return (
    <>
      <Field label="Name" layout="row">
        <Input
          size="sm"
          value={name ?? project.name}
          onChange={(e) => setName(e.target.value)}
          onBlur={commitName}
          onKeyDown={(e) => e.key === "Enter" && commitName()}
        />
      </Field>
      <Field label="Colour" layout="row">
        <Segmented
          aria-label="Project colour"
          size="sm"
          className="flex-wrap"
          options={SWATCHES.map((s) => ({ value: s, label: s }))}
          value={project.color}
          onValueChange={(color) => void patchProject(projectId, { color })}
        />
      </Field>
    </>
  );
}

export function ProjectSettings({ projectId }: { projectId: ProjectId; compact?: boolean }): ReactElement {
  const scope = projectScope(projectId);
  const costOverview = useSetting(appScope(), "cost_overview") ?? false;
  const root = projectsStore.useSelector((s) => s.byId[projectId]?.root);
  return (
    <div className="flex flex-col gap-5">
      <Section title="Project">
        <ProjectFields projectId={projectId} />
      </Section>
      <Section title="Memory">
        <MemoryLocationField projectId={projectId} />
      </Section>
      <Section title="Worktrees">
        <WorktreeFields projectId={projectId} root={root} />
      </Section>
      <Section title="Automation">
        <AutomationFields projectId={projectId} />
      </Section>
      <Section title="Detail level">
        <SegmentedSetting
          scope={scope}
          settingKey="detail_level"
          label="Card detail"
          inherit
          options={DETAIL_LEVELS.map((d) => ({ value: d, label: d }))}
        />
      </Section>
      {costOverview && (
        <Section title="Costs">
          <CostsPanel scope={scope} />
        </Section>
      )}
    </div>
  );
}
