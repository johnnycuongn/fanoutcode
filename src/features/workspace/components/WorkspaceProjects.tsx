// The workspace's projects: colour, name, root, a Missing badge, move up/down, Relocate… and Remove; Add project…
import { ArrowDown, ArrowUp } from "lucide-react";
import type { ReactElement } from "react";
import { Badge, Button, IconButton } from "@/components/ui";
import type { ProjectId } from "@/lib/bindings";
import { projectsStore } from "@/lib/stores/projectsStore";
import { asSwatch } from "@/lib/format";
import { swatchClass } from "@/lib/theme";
import { pickFolder } from "../pickers";
import { workspaceStore } from "../store";
import { Section } from "./Section";

export function WorkspaceProjects(): ReactElement | null {
  const ws = workspaceStore.useSelector((s) => s.workspace);
  const status = workspaceStore.useSelector((s) => s.status);
  const projects = projectsStore.useSelector((s) => s.byId);
  if (!ws) return null;
  const ids = ws.projects.map((p) => p.project_id);
  const move = (i: number, by: number) => {
    const next = [...ids];
    [next[i], next[i + by]] = [next[i + by], next[i]];
    void workspaceStore.reorder(next);
  };
  const relocate = async (id: ProjectId) => {
    const path = await pickFolder("Where is this project now?");
    if (path) await workspaceStore.relocateProject(id, path);
  };
  const add = async () => {
    const path = await pickFolder("Add a project folder");
    if (path) await workspaceStore.addProject(path);
  };
  return (
    <Section title="Projects">
      <ul className="flex flex-col gap-1">
        {ids.map((id, i) => {
          const p = projects[id];
          const missing = status?.missing_projects.includes(id);
          return (
            <li key={id} className="flex items-center gap-2 text-sm">
              <span aria-hidden className={`size-2 shrink-0 rounded-full ${swatchClass(asSwatch(p?.color), "bg")}`} />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-ink">{p?.name ?? id}</span>
                <span className="block truncate font-mono text-xs text-mute">{p?.root}</span>
              </span>
              {missing && (
                <Badge tone="warn" size="sm">
                  Missing
                </Badge>
              )}
              <IconButton
                size="sm"
                label="Move up"
                icon={<ArrowUp className="size-3.5" />}
                disabled={i === 0}
                onClick={() => move(i, -1)}
              />
              <IconButton
                size="sm"
                label="Move down"
                icon={<ArrowDown className="size-3.5" />}
                disabled={i === ids.length - 1}
                onClick={() => move(i, 1)}
              />
              <Button size="sm" variant="ghost" onClick={() => void relocate(id)}>
                Relocate…
              </Button>
              <Button size="sm" variant="ghost" onClick={() => void workspaceStore.removeProject(id)}>
                Remove
              </Button>
            </li>
          );
        })}
      </ul>
      <div>
        <Button size="sm" variant="secondary" onClick={() => void add()}>
          Add project…
        </Button>
      </div>
    </Section>
  );
}
