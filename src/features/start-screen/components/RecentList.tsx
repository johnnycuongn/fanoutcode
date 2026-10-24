// One list of recent projects or workspaces; a missing one offers Relocate… (and, for a workspace, Re-export…).
import type { ReactElement } from "react";
import { Badge, Button } from "@/components/ui";
import type { RecentEntry } from "@/lib/bindings";
import { openRecent, reexportWorkspace, relocateRecent } from "../actions";

export function RecentList({ title, entries }: { title: string; entries: RecentEntry[] }): ReactElement {
  return (
    <section aria-label={title} className="flex flex-col gap-1">
      <h2 className="text-xs font-medium text-mute">{title}</h2>
      {entries.length === 0 && <p className="text-sm text-mute">Nothing yet.</p>}
      {entries.map((e) => (
        <div key={`${e.kind}:${e.id}`} className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={e.missing}
            className="min-w-0 flex-1 justify-start gap-2"
            onClick={() => void openRecent(e)}
          >
            <span className="truncate font-medium">{e.name}</span>
            <span className="truncate text-xs text-mute">{e.path}</span>
          </Button>
          {e.missing && (
            <>
              <Badge tone="warn" size="sm">
                Missing
              </Badge>
              <Button variant="secondary" size="sm" onClick={() => void relocateRecent(e)}>
                Relocate…
              </Button>
              {e.kind === "Workspace" && (
                <Button variant="secondary" size="sm" onClick={() => void reexportWorkspace(e)}>
                  Re-export…
                </Button>
              )}
            </>
          )}
        </div>
      ))}
    </section>
  );
}
