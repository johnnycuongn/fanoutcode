// The workspace settings screen: file, projects, linking policy, options, prompt fragments, agents, workspace
// references, the Jira default site and costs. Plan 06's settings dialog mounts it for workspace windows.
import { useEffect, type ReactElement, type ReactNode } from "react";
import type { WorkspaceId } from "@/lib/bindings";
import { workspaceStore } from "../store";
import { useSetting, workspaceScope } from "@/features/settings";
import { ContextDocsEditor } from "./ContextDocsEditor";
import { JiraLinkForm } from "./JiraLinkForm";
import { LinkingPolicy } from "./LinkingPolicy";
import { PromptFragments } from "./PromptFragments";
import { Section } from "./Section";
import { WorkspaceAgents } from "./WorkspaceAgents";
import { WorkspaceFileSection } from "./WorkspaceFileSection";
import { WorkspaceOptions } from "./WorkspaceOptions";
import { WorkspaceProjects } from "./WorkspaceProjects";

export type WorkspaceSettingsProps = { workspaceId: WorkspaceId; jira?: ReactNode; costs?: ReactNode };

export function WorkspaceSettings({ workspaceId, jira, costs }: WorkspaceSettingsProps): ReactElement {
  const loaded = workspaceStore.useSelector((s) => s.workspace?.id === workspaceId);
  const site = useSetting(workspaceScope(workspaceId), "jira");
  useEffect(() => {
    if (!loaded) void workspaceStore.load(workspaceId);
  }, [loaded, workspaceId]);
  return (
    <div className="flex flex-col gap-3">
      <WorkspaceFileSection />
      <WorkspaceProjects />
      <LinkingPolicy />
      <WorkspaceOptions workspaceId={workspaceId} />
      <PromptFragments />
      <WorkspaceAgents />
      <Section title="Workspace references">
        <ContextDocsEditor container={{ type: "Workspace", id: workspaceId }} />
      </Section>
      <Section title="Jira default site">
        {jira ?? <JiraLinkForm scope={{ workspace: workspaceId }} linked={site ?? null} />}
      </Section>
      {costs && <Section title="Costs">{costs}</Section>}
    </div>
  );
}
