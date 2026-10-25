// A file on the hub graph (§10.7 "Nodes"): its short path (full path on hover), net change and its owners' avatars;
// a file two agents edit carries the warning border.
import type { ReactElement } from "react";
import { Avatar, Tooltip, cx } from "@/components/ui";
import type { FileNodeData, ViewOptions } from "../../types";
import { CARD, RepoBadge, netChange, shortPath } from "./cardParts";

export function FileCard({
  data,
  view,
}: {
  data: FileNodeData & { ownerRoles: Record<string, string> };
  view: ViewOptions;
}): ReactElement {
  const shared = data.owners.length >= 2;
  return (
    <div
      data-node-kind="file"
      data-shared={shared ? "true" : "false"}
      className={cx(CARD, "flex w-50 flex-col gap-1 px-2 py-1.5", shared ? "border-warn" : "border-line")}
    >
      <div className="flex min-w-0 items-center gap-1.5">
        <Tooltip content={data.path}>
          <span tabIndex={0} className="min-w-0 flex-1 truncate font-mono text-xs">
            {shortPath(data.path)}
          </span>
        </Tooltip>
        {view.showRepo && <RepoBadge repo={data.repo} roots={view.roots} />}
      </div>
      <div className="flex items-center gap-1">
        <span className="font-mono text-[11px] text-mute">{netChange(data.added, data.removed)}</span>
        <span className="ml-auto flex -space-x-1">
          {data.owners.map((o) => (
            <Avatar key={o} name={data.ownerRoles[o] ?? "?"} size="sm" />
          ))}
        </span>
      </div>
    </div>
  );
}
