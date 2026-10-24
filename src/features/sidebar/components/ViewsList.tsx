// The window's views at the top of the Tasks section, each with its amber needs-you count; click activates one.
import type { ReactElement } from "react";
import { Badge, Button, cx } from "@/components/ui";
import type { ProjectId } from "@/lib/bindings";
import { projectsStore } from "@/lib/stores/projectsStore";
import { tasksStore } from "@/lib/stores/tasksStore";
import { swatchClass } from "@/lib/theme";
import { setActiveTab, shellStore } from "@/features/shell";
import { tabNeedsYouCount, viewProjectIds, viewsStore } from "@/features/views";

/** With `projectId`, only the views that show one of that project's tasks. */
export function ViewsList({ projectId }: { projectId?: ProjectId }): ReactElement | null {
  const views = viewsStore.useSelector((s) => s.views);
  const tasksById = tasksStore.useSelector((s) => s.byId);
  const projectsById = projectsStore.useSelector((s) => s.byId);
  const activeTab = shellStore.useSelector((s) => s.activeTab);
  const workspace = shellStore.useSelector((s) => s.kind === "workspace");
  const shown = projectId ? views.filter((v) => viewProjectIds(v, tasksById).includes(projectId)) : views;
  if (shown.length === 0) return null;
  return (
    <nav aria-label="Views" className="flex flex-col py-1">
      {shown.map((view) => {
        const count = tabNeedsYouCount(view, tasksById);
        return (
          <Button
            key={view.id}
            variant="ghost"
            size="sm"
            aria-current={view.id === activeTab ? "page" : undefined}
            className={cx("w-full justify-start gap-1.5 px-2", view.id === activeTab && "bg-well")}
            onClick={() => setActiveTab(view.id)}
          >
            {workspace &&
              viewProjectIds(view, tasksById).map((id) => (
                <span
                  key={id}
                  aria-hidden
                  className={cx("size-1.5 rounded-full", swatchClass(projectsById[id]?.color ?? "slate", "bg"))}
                />
              ))}
            <span className="min-w-0 flex-1 truncate text-left">{view.name}</span>
            {count > 0 && (
              <Badge tone="warn" size="sm" data-tone="warn">
                {count}
              </Badge>
            )}
          </Button>
        );
      })}
    </nav>
  );
}
