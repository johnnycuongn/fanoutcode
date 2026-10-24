// The title row: draggable, room for the traffic lights, the project or workspace name and the unsaved dot.
import type { ReactElement } from "react";

export function WindowChrome({ title, dirty }: { title: string; dirty: boolean }): ReactElement {
  return (
    <header
      data-tauri-drag-region
      className="flex h-9 shrink-0 select-none items-center justify-center gap-1 bg-bar pl-20 pr-3 text-sm text-ink"
    >
      <span data-tauri-drag-region className="truncate font-medium">
        {title}
      </span>
      {dirty && (
        <span aria-label="Unsaved workspace changes" className="text-warn">
          •
        </span>
      )}
    </header>
  );
}
