import type { ReactElement } from "react";
import { Avatar } from "@/components/ui";
import type { TeamMember } from "../types";

/** One small avatar per agent on the task, with a pulsing dot while it works. */
export function CardTeamRow({ team }: { team: TeamMember[] }): ReactElement | null {
  if (team.length === 0) return null;
  return (
    <div className="flex items-center gap-1.5">
      {team.map((member) => (
        <span
          key={member.agentId ?? member.role}
          data-testid="team-member"
          data-working={member.working}
          title={member.role}
          className="inline-flex"
        >
          <Avatar name={member.role} size="xs" status={member.working ? "working" : "idle"} />
        </span>
      ))}
    </div>
  );
}
