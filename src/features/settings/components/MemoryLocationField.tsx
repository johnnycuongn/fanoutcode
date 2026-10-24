// Where the project's task memory folders live: app storage or a folder. A `Project` field written through
// `projects_update` (not a setting, A12).
import type { ReactElement } from "react";
import { Button, Field, toast } from "@/components/ui";
import { commands, type MemoryLocation, type ProjectId } from "@/lib/bindings";
import { projectsStore, upsertProject } from "@/lib/stores/projectsStore";

async function writeLocation(projectId: ProjectId, memory_location: MemoryLocation) {
  const r = await commands.projectsUpdate(projectId, { name: null, color: null, memory_location });
  if (r?.status === "ok") upsertProject(r.data);
  else if (r) toast.error(r.error);
}

export function MemoryLocationField({ projectId }: { projectId: ProjectId }): ReactElement {
  const location = projectsStore.useSelector((s) => s.byId[projectId]?.memory_location);
  const choose = async () => {
    const r = await commands.memoryPickLocation(projectId);
    if (r?.status === "ok" && r.data) await writeLocation(projectId, { type: "Folder", path: r.data });
    else if (r?.status === "error") toast.error(r.error);
  };
  return (
    <Field
      label="Memory location"
      layout="row"
      hint="Memory files stay readable without Team Code. Inside the repo, add the folder to .gitignore or keep it tracked; the app does not decide."
    >
      <p className="truncate font-mono text-xs text-ink">
        {location?.type === "Folder" ? location.path : "App storage"}
      </p>
      <div className="flex gap-2">
        <Button size="sm" variant="secondary" onClick={() => void choose()}>
          Choose folder…
        </Button>
        {location?.type === "Folder" && (
          <Button size="sm" variant="ghost" onClick={() => void writeLocation(projectId, { type: "AppStorage" })}>
            Use app storage
          </Button>
        )}
      </div>
    </Field>
  );
}
