// An agent on the canvas (§10.7 "Nodes"): its role, a general-purpose tag for ad-hoc roles, the working ring, and
// its cost when the cost overview is on.
import type { ReactElement } from "react";
import { Chip, cx } from "@/components/ui";
import { MiniMarkdown } from "@/livespec/MiniMarkdown";
import type { AgentNodeData, ViewOptions } from "../../types";
import { CARD, costText } from "./cardParts";
import { WorkingRing } from "./WorkingRing";

export function AgentHub({ data, view }: { data: AgentNodeData; view: ViewOptions }): ReactElement {
  return (
    <div
      data-node-kind="agent"
      data-working={data.working ? "true" : "false"}
      className={cx(CARD, "flex w-40 items-center gap-2 px-2.5 py-2", data.working ? "border-ok" : "border-line")}
    >
      {data.working && <WorkingRing size={22} />}
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <MiniMarkdown text={data.role} className="truncate text-sm font-medium [&_p]:mb-0 [&_p]:truncate" />
        <div className="flex items-center gap-1">
          {data.adhoc && (
            <Chip size="sm" tone="neutral">
              general-purpose
            </Chip>
          )}
          {view.showCost && data.costUsd !== null && (
            <span className="ml-auto text-xs text-mute tabular-nums">{costText(data.costUsd)}</span>
          )}
        </div>
      </div>
    </div>
  );
}
