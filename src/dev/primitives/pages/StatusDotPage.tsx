import type { ReactElement } from "react";
import { STAGES, StatusDot, type AgentStatus } from "../../../components/ui";

const statuses: AgentStatus[] = ["working", "needs-you", "idle"];

export default function StatusDotPage(): ReactElement {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">StatusDot</h1>
      <section className="flex items-center gap-4">
        {statuses.map((status) => (
          <span key={status} className="flex items-center gap-2 text-sm">
            <StatusDot status={status} size="sm" />
            <StatusDot status={status} />
            {status}
          </span>
        ))}
      </section>
      <section className="flex items-center gap-4">
        {STAGES.map((stage) => (
          <span key={stage} className="flex items-center gap-2 text-sm">
            <StatusDot stage={stage} label={stage} />
            {stage}
          </span>
        ))}
      </section>
      <p className="text-sm text-mute">The working pulse stops under macOS Reduce motion.</p>
    </div>
  );
}
