// A workspace project's root in the multi-root sidebar: colour dot, name and needs-you count; collapsible; marked
// and scrolled into view when it is the focused project (⌘1–⌘9).
import { ChevronDown, ChevronRight } from "lucide-react";
import { useEffect, useRef, useState, type ReactElement, type ReactNode } from "react";
import { Badge, Button, cx } from "@/components/ui";
import type { Project } from "@/lib/bindings";
import { swatchClass } from "@/lib/theme";

export type ProjectRootProps = {
  project: Project;
  needsYou: number;
  defaultOpen?: boolean;
  focused?: boolean;
  children: ReactNode;
};

export function ProjectRoot({
  project,
  needsYou,
  defaultOpen = true,
  focused,
  children,
}: ProjectRootProps): ReactElement {
  const [open, setOpen] = useState(defaultOpen);
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    if (focused) ref.current?.scrollIntoView({ block: "nearest" });
  }, [focused]);
  const Chevron = open ? ChevronDown : ChevronRight;
  return (
    <section
      ref={ref}
      data-project-root={project.id}
      data-focused={focused ? "true" : undefined}
      aria-label={project.name}
      className={cx("border-l-2", focused ? "border-accent bg-well/50" : "border-transparent")}
    >
      <Button
        variant="ghost"
        size="sm"
        aria-expanded={open}
        className="w-full justify-start gap-1.5 px-1.5 font-medium"
        onClick={() => setOpen((o) => !o)}
      >
        <Chevron aria-hidden className="size-3.5 text-mute" />
        <span aria-hidden className={cx("size-2 shrink-0 rounded-full", swatchClass(project.color, "bg"))} />
        <span className="min-w-0 flex-1 truncate text-left">{project.name}</span>
        {needsYou > 0 && (
          <Badge tone="warn" size="sm" data-tone="warn">
            {needsYou}
          </Badge>
        )}
      </Button>
      {open && <div className="pb-1">{children}</div>}
    </section>
  );
}
