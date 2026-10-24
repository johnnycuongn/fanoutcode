// The strip under the views listing every task that needs you, newest first (hidden on the Board, which shows the
// same thing). In workspace windows each chip carries its home project's colour. The view's cost sits at the end
// when the cost overview is on.
import { AnimatePresence, motion } from "motion/react";
import type { ReactElement } from "react";
import { Chip, cx } from "@/components/ui";
import { handbackLabel } from "@/lib/format";
import { transitionFor, useReducedMotion } from "@/lib/motion";
import { projectsStore } from "@/lib/stores/projectsStore";
import { needsYouTasks, selectTaskList, tasksStore } from "@/lib/stores/tasksStore";
import { swatchClass } from "@/lib/theme";
import { appScope, useSetting } from "@/features/settings";
import { openTaskInCurrentView } from "../openTask";
import { shellStore } from "../store";
import { ViewCostBadge } from "./ViewCostBadge";

export function AttentionStrip(): ReactElement | null {
  const activeTab = shellStore.useSelector((s) => s.activeTab);
  const workspace = shellStore.useSelector((s) => s.kind === "workspace");
  const tasks = tasksStore.useSelector(selectTaskList);
  const projectsById = projectsStore.useSelector((s) => s.byId);
  const costOverview = useSetting(appScope(), "cost_overview") ?? false;
  const reduced = useReducedMotion();
  const waiting = needsYouTasks(tasks);
  if (activeTab === "board" || (waiting.length === 0 && !costOverview)) return null;
  return (
    <div
      role="toolbar"
      aria-label="Needs you"
      className="flex h-8 shrink-0 items-center gap-1.5 overflow-x-auto border-t border-line bg-bar px-2"
    >
      <AnimatePresence initial={false}>
        {waiting.map((task) => (
          <motion.div
            key={task.id}
            layout
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={transitionFor(reduced)}
          >
            <Chip
              size="sm"
              tone="warn"
              icon={
                workspace ? (
                  <span
                    aria-hidden
                    className={cx(
                      "size-1.5 rounded-full",
                      swatchClass(projectsById[task.project_id]?.color ?? "slate", "bg"),
                    )}
                  />
                ) : undefined
              }
              onClick={() => openTaskInCurrentView(task.id)}
            >
              {`${task.title} · ${handbackLabel(task)}`}
            </Chip>
          </motion.div>
        ))}
      </AnimatePresence>
      {costOverview && (
        <div className="ml-auto">
          <ViewCostBadge viewId={activeTab} />
        </div>
      )}
    </div>
  );
}
