// Small pieces the three cards share: short paths, net change, cost and the repo badge.
import type { ReactElement } from "react";
import { Badge } from "@/components/ui";
import { asSwatch, formatUsd } from "@/lib/format";
import type { RepoRoot } from "../../types";

/** The last two segments of a path (`src/a.ts`). */
export function shortPath(path: string): string {
  return path.split("/").filter(Boolean).slice(-2).join("/");
}

/** `+3 −1`, with a real minus sign. */
export function netChange(added: number, removed: number): string {
  return `+${added} −${removed}`;
}

export function costText(costUsd: number): string {
  return formatUsd(costUsd);
}

export function RepoBadge({ repo, roots }: { repo: string | null; roots: RepoRoot[] }): ReactElement | null {
  const root = roots.find((r) => r.root === repo);
  if (!root) return null;
  return (
    <Badge size="sm" swatch={asSwatch(root.color)} className="shrink-0">
      {root.name}
    </Badge>
  );
}

export const CARD = "rounded-card border bg-card text-ink shadow-card";
