// The LiveSpec level for the task's project: Off, Simple or Heavy, remembered per project (§6.1).
import type { ReactElement } from "react";
import { Segmented } from "@/components/ui";
import type { LiveSpecLevel } from "@/lib/bindings";

const OPTIONS = [
  { value: "Off", label: "Off" },
  { value: "Simple", label: "Simple" },
  { value: "Heavy", label: "Heavy" },
] as const satisfies readonly { value: LiveSpecLevel; label: string }[];

export function LiveSpecSwitch({
  level,
  onChange,
  disabled = false,
}: {
  level: LiveSpecLevel;
  onChange(level: LiveSpecLevel): void;
  disabled?: boolean;
}): ReactElement {
  return (
    <div className="flex items-center gap-2 text-xs text-mute">
      <span aria-hidden>LiveSpec</span>
      <Segmented
        aria-label="LiveSpec"
        size="sm"
        value={level}
        onValueChange={onChange}
        options={OPTIONS.map((o) => ({ ...o, disabled }))}
      />
    </div>
  );
}
