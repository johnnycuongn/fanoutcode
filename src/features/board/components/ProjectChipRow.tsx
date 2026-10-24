import type { ReactElement } from "react";
import { Chip, cx } from "@/components/ui";
import { swatchClass } from "@/lib/theme";
import { selectNeedsYouCount, selectProjectCounts, setFilters, useBoard } from "../store";

const toggle = (list: string[], id: string): string[] =>
  list.includes(id) ? list.filter((x) => x !== id) : [...list, id];

/** Mixed boards: one chip per project (a filter, with its card count) and a Needs you chip. */
export function ProjectChipRow(): ReactElement {
  const counts = useBoard(selectProjectCounts);
  const projects = useBoard((s) => s.projects);
  const chosen = useBoard((s) => s.filters.projects);
  const needsYou = useBoard((s) => s.filters.needsYou);
  const needsYouCount = useBoard(selectNeedsYouCount);
  const focused = useBoard((s) => s.focusedProject);
  return (
    <div
      role="toolbar"
      aria-label="Projects"
      className="flex flex-wrap items-center gap-1.5 border-b border-line px-3 py-2"
    >
      {counts.map(({ projectId, count }) => (
        <Chip
          key={projectId}
          size="sm"
          selected={chosen.includes(projectId)}
          count={count}
          data-focused={focused === projectId}
          icon={
            <span
              aria-hidden
              className={cx("size-2 rounded-full", swatchClass(projects[projectId]?.color ?? "", "bg"))}
            />
          }
          onClick={() => setFilters({ projects: toggle(chosen, projectId) })}
        >
          {projects[projectId]?.name ?? projectId}
        </Chip>
      ))}
      <Chip
        size="sm"
        tone="warn"
        selected={needsYou}
        count={needsYouCount}
        onClick={() => setFilters({ needsYou: !needsYou })}
      >
        Needs you
      </Chip>
    </div>
  );
}
