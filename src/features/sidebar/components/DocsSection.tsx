// The Docs section: where the project's memory lives, its context docs (Plan 10 supplies them), and a drop target
// that forwards dropped file paths to `onAddPath`.
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { FileText, Folder, Globe, BookOpen } from "lucide-react";
import { useEffect, type ReactElement } from "react";
import { Panel } from "@/components/ui";
import type { ProjectId } from "@/lib/bindings";
import { projectsStore } from "@/lib/stores/projectsStore";
import type { ContextDocItem } from "../types";

const ICON = { file: FileText, folder: Folder, url: Globe, confluence: BookOpen } as const;

function useDroppedPaths(onAddPath?: (path: string) => void) {
  useEffect(() => {
    if (!onAddPath) return;
    let off: (() => void) | undefined;
    let live = true;
    try {
      void getCurrentWebview()
        .onDragDropEvent((e) => {
          if (e.payload.type === "drop") for (const p of e.payload.paths) onAddPath(p);
        })
        .then((fn) => (live ? (off = fn) : fn()))
        .catch(() => {});
    } catch {
      // Outside Tauri there is no webview to listen on.
    }
    return () => {
      live = false;
      off?.();
    };
  }, [onAddPath]);
}

export type DocsSectionProps = { projectId?: ProjectId; docs?: ContextDocItem[]; onAddPath?: (path: string) => void };

export function DocsSection({ projectId, docs = [], onAddPath }: DocsSectionProps): ReactElement {
  const memory = projectsStore.useSelector((s) => (projectId ? s.byId[projectId]?.memory_location : undefined));
  useDroppedPaths(onAddPath);
  return (
    <div className="flex flex-col gap-3 px-2">
      <div>
        <h3 className="py-1 text-xs font-medium text-mute">Memory</h3>
        <p className="truncate font-mono text-xs text-ink" title={memory?.type === "Folder" ? memory.path : undefined}>
          {memory?.type === "Folder" ? memory.path : "App storage"}
        </p>
      </div>
      <div>
        <h3 className="py-1 text-xs font-medium text-mute">Context docs</h3>
        <ul className="flex flex-col gap-0.5">
          {docs.map((d) => {
            const Icon = ICON[d.kind];
            return (
              <li key={d.id} className="flex items-center gap-1.5 text-sm text-ink" title={d.href}>
                <Icon aria-hidden className="size-3.5 shrink-0 text-mute" />
                <span className="truncate">{d.label}</span>
              </li>
            );
          })}
        </ul>
      </div>
      <Panel variant="well" className="border border-dashed border-line text-center text-xs text-mute">
        Drag a file in to add it
      </Panel>
    </div>
  );
}
