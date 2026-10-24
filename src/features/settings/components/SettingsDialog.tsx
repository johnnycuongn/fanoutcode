// ⌘,: Settings, with an App tab, a Project tab (in a workspace window with a project picker) and a Workspace tab that
// Plan 10 fills through `workspaceTab`.
import { useState, type ReactElement, type ReactNode } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  Field,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui";
import type { Container, Project } from "@/lib/bindings";
import { closeSettings, settingsDialogStore, type SettingsTab } from "../store";
import { AppSettings } from "./AppSettings";
import { ProjectSettings } from "./ProjectSettings";

export type SettingsDialogProps = { container: Container | null; projects: Project[]; workspaceTab?: ReactNode };

function ProjectTab({ container, projects }: Omit<SettingsDialogProps, "workspaceTab">): ReactElement | null {
  const [picked, setPicked] = useState<string | null>(null);
  if (container?.type === "Project") return <ProjectSettings projectId={container.id} />;
  const id = picked ?? projects[0]?.id;
  if (!id) return null;
  return (
    <div className="flex flex-col gap-3">
      <Field label="Project" layout="row">
        <Select value={id} onValueChange={setPicked}>
          <SelectTrigger size="sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {projects.map((p) => (
              <SelectItem key={p.id} value={p.id}>
                {p.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <ProjectSettings key={id} projectId={id} />
    </div>
  );
}

export function SettingsDialog({ container, projects, workspaceTab }: SettingsDialogProps): ReactElement {
  const { open, tab } = settingsDialogStore.use();
  const setTab = (t: string) => settingsDialogStore.set({ tab: t as SettingsTab });
  return (
    <Dialog open={open} onOpenChange={(o) => !o && closeSettings()}>
      <DialogContent size="lg" className="max-h-[85vh] overflow-auto">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
        </DialogHeader>
        <Tabs value={tab} onValueChange={setTab}>
          <TabsList aria-label="Settings scope">
            <TabsTrigger value="app">App</TabsTrigger>
            {container && <TabsTrigger value="project">Project</TabsTrigger>}
            {workspaceTab && <TabsTrigger value="workspace">Workspace</TabsTrigger>}
          </TabsList>
          <TabsContent value="app" className="pt-3">
            <AppSettings />
          </TabsContent>
          <TabsContent value="project" className="pt-3">
            <ProjectTab container={container} projects={projects} />
          </TabsContent>
          {workspaceTab && (
            <TabsContent value="workspace" className="pt-3">
              {workspaceTab}
            </TabsContent>
          )}
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
