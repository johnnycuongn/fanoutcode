// The linking policy: read and modify on request, or read only; per-project protected targets under the former.
import type { ReactElement } from "react";
import { Segmented, Switch } from "@/components/ui";
import type { LinkPolicy, ProjectId } from "@/lib/bindings";
import { projectsStore } from "@/lib/stores/projectsStore";
import { POLICY_APPLIES_NOTE, READ_AND_MODIFY_LABEL, READ_ONLY_LABEL } from "../linking";
import { workspaceStore } from "../store";
import { Section } from "./Section";

const OPTIONS = [
  { value: "ReadAndModifyOnRequest" as const, label: READ_AND_MODIFY_LABEL },
  { value: "ReadOnly" as const, label: READ_ONLY_LABEL },
];

export function LinkingPolicy(): ReactElement | null {
  const ws = workspaceStore.useSelector((s) => s.workspace);
  const names = projectsStore.useSelector((s) => s.byId);
  if (!ws) return null;
  const readOnly = ws.policy === "ReadOnly";
  const protectedIds = ws.projects.filter((p) => p.protected).map((p) => p.project_id);
  const setPolicy = (policy: LinkPolicy) => void workspaceStore.setPolicy(policy, protectedIds);
  const toggle = (id: ProjectId, on: boolean) =>
    void workspaceStore.setPolicy(ws.policy, on ? [...protectedIds, id] : protectedIds.filter((p) => p !== id));
  return (
    <Section title="Linking policy">
      <Segmented aria-label="Linking policy" options={OPTIONS} value={ws.policy} onValueChange={setPolicy} />
      <ul className="flex flex-col gap-1">
        {ws.projects.map((p) => (
          <li key={p.project_id} className="flex items-center justify-between gap-2 text-sm">
            <span className="truncate text-ink">{names[p.project_id]?.name ?? p.project_id}</span>
            <span className="flex items-center gap-2">
              {readOnly && <span className="text-xs text-mute">Already read only</span>}
              <Switch
                label="Protected target"
                size="sm"
                checked={readOnly || p.protected}
                disabled={readOnly}
                onCheckedChange={(on) => toggle(p.project_id, on)}
              />
            </span>
          </li>
        ))}
      </ul>
      <p className="text-xs text-mute">{POLICY_APPLIES_NOTE}</p>
    </Section>
  );
}
