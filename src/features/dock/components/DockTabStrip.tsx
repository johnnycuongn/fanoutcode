// The dock's tab strip: the stage's visible tabs, and the grow toggle.
import { Maximize2, Minimize2 } from "lucide-react";
import type { ReactElement, ReactNode } from "react";
import { IconButton, TabsList, TabsTrigger } from "@/components/ui";
import { DOCK_TABS } from "../tabRules";
import type { DockTab } from "../types";

export interface DockTabStripProps {
  tabs: DockTab[];
  grown: boolean;
  onToggleGrow(): void;
  /** Extra content per tab label, e.g. the Changes count. */
  counts?: Partial<Record<DockTab, number>>;
  trailing?: ReactNode;
}

export function DockTabStrip({ tabs, grown, onToggleGrow, counts = {}, trailing }: DockTabStripProps): ReactElement {
  return (
    <div className="flex shrink-0 items-center gap-1 border-b border-line bg-bar pr-1">
      <TabsList variant="bar" size="sm" aria-label="Dock" className="flex-1">
        {DOCK_TABS.filter((t) => tabs.includes(t.id)).map((t) => (
          <TabsTrigger key={t.id} value={t.id} {...(counts[t.id] ? { count: counts[t.id] } : {})}>
            {t.label}
          </TabsTrigger>
        ))}
      </TabsList>
      {trailing}
      <IconButton
        size="sm"
        label={grown ? "Shrink dock" : "Grow dock"}
        icon={grown ? <Minimize2 aria-hidden className="size-3.5" /> : <Maximize2 aria-hidden className="size-3.5" />}
        onClick={onToggleGrow}
      />
    </div>
  );
}
