// The project's own test command when none is set (`projects_detect_test`): Settings shows where it came from, and the
// stage stepper marks Test skipped only when there is neither a set nor a detected command.
import { useEffect, useState } from "react";
import { commands, type DetectedTest, type ProjectId } from "@/lib/bindings";

/** null while unknown, when there is none, or when it cannot be fetched. */
export function useDetectedTest(projectId: ProjectId | null): DetectedTest | null {
  const [detected, setDetected] = useState<DetectedTest | null>(null);
  useEffect(() => {
    if (!projectId) return;
    let live = true;
    Promise.resolve()
      .then(() => commands.projectsDetectTest(projectId))
      .then((r) => live && setDetected(r?.status === "ok" ? r.data : null))
      .catch(() => live && setDetected(null));
    return () => {
      live = false;
    };
  }, [projectId]);
  return detected;
}
