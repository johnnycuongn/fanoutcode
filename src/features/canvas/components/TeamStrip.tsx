// The one-line team strip the Swap placement shows while the terminal fills the pane (§10.7 "Placement").
import { useEffect, type ReactElement } from "react";
import { teamSummary } from "../placement";
import { closeCanvas, openCanvas, useCanvas } from "../store";
import { WorkingRing } from "./cards/WorkingRing";

export function TeamStrip({ taskId }: { taskId: string }): ReactElement {
  const s = useCanvas(taskId);
  useEffect(() => {
    void openCanvas(taskId);
    return () => closeCanvas(taskId);
  }, [taskId]);
  return (
    <div
      data-team-strip={taskId}
      className="flex h-7 items-center gap-2 border-t border-line bg-bar px-3 text-xs text-mute"
    >
      {s.model.agents.some((a) => a.working) && <WorkingRing size={12} />}
      <span className="truncate">{teamSummary(s.model)}</span>
    </div>
  );
}
