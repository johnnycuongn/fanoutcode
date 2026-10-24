// The workspace Rail layer: a narrow vertical rail of projects (initial on its colour, with its needs-you count).
// Choosing one focuses it, so the sidebar and the Board show that project; All returns to the swimlane board.
import { Layers } from "lucide-react";
import type { ReactElement } from "react";
import { Badge, Tabs, TabsList, TabsTrigger, cx } from "@/components/ui";
import { projectsStore, selectProjectList } from "@/lib/stores/projectsStore";
import { selectTaskList, tasksStore } from "@/lib/stores/tasksStore";
import { swatchClass } from "@/lib/theme";
import { setFocusedProject, shellStore } from "../store";

const ALL = "all";

export function ProjectRail(): ReactElement {
  const focused = shellStore.useSelector((s) => s.focusedProject);
  const projects = projectsStore.useSelector(selectProjectList);
  const tasks = tasksStore.useSelector(selectTaskList);
  const choose = (v: string) => setFocusedProject(v === ALL ? null : v);
  return (
    <Tabs value={focused ?? ALL} onValueChange={choose} orientation="vertical" activationMode="manual">
      <TabsList
        aria-label="Projects"
        className="flex h-full w-12 shrink-0 flex-col items-center gap-1 border-r border-line bg-bar py-2"
      >
        <TabsTrigger value={ALL} aria-label="All projects" title="All projects" onClick={() => choose(ALL)}>
          <Layers aria-hidden className="size-4" />
        </TabsTrigger>
        {projects.map((p, i) => {
          const count = tasks.filter((t) => t.project_id === p.id && !t.archived && t.state.type === "NeedsYou").length;
          return (
            <TabsTrigger
              key={p.id}
              value={p.id}
              aria-label={p.name}
              title={`${p.name} (⌘${i + 1})`}
              className="relative"
              onClick={() => choose(p.id)}
            >
              <span
                aria-hidden
                className={cx(
                  "flex size-7 items-center justify-center rounded-card font-medium text-accent-ink",
                  swatchClass(p.color, "bg"),
                )}
              >
                {p.name.slice(0, 1).toUpperCase()}
              </span>
              {count > 0 && (
                <Badge tone="warn" variant="solid" size="sm" data-tone="warn" className="absolute -top-1 -right-1">
                  {count}
                </Badge>
              )}
            </TabsTrigger>
          );
        })}
      </TabsList>
    </Tabs>
  );
}
