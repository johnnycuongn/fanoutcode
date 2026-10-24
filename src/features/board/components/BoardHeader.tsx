import type { ReactElement, ReactNode } from "react";
import { FilterBar } from "./FilterBar";

/** Left: the mode control (project picker) and filters. Right: the board's cost total. */
export function BoardHeader({
  modeControl,
  trailing,
}: {
  modeControl?: ReactNode;
  trailing?: ReactNode;
}): ReactElement {
  return (
    <header className="flex items-center gap-3 border-b border-line bg-bar px-3 py-2">
      {modeControl}
      <FilterBar />
      <div className="ml-auto flex items-center gap-2 text-sm text-mute">{trailing}</div>
    </header>
  );
}
