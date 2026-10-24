import type { ReactElement } from "react";
import { Avatar, AvatarGroup } from "../../../components/ui";

const ROLES = ["frontend", "backend", "best-practice reviewer", "codebase-consistency reviewer", "security reviewer"];
const SWATCH = ["violet", "indigo", "emerald", "orange", "rose"] as const;

export default function AvatarPage(): ReactElement {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">Avatar</h1>
      {(["xs", "sm", "md", "lg"] as const).map((size) => (
        <section key={size} className="flex items-center gap-3">
          <span className="w-8 text-sm text-mute">{size}</span>
          {ROLES.map((role, i) => (
            <Avatar key={role} name={role} size={size} swatch={SWATCH[i]} />
          ))}
        </section>
      ))}
      <section className="flex items-center gap-3">
        <span className="w-8 text-sm text-mute">status</span>
        <Avatar name="frontend" size="lg" status="working" swatch="violet" />
        <Avatar name="backend" size="lg" status="needs-you" swatch="indigo" />
        <Avatar name="security reviewer" size="lg" status="idle" />
        <Avatar name="frontend" size="lg" shape="square" swatch="emerald" />
      </section>
      <section className="flex items-center gap-3">
        <span className="w-8 text-sm text-mute">group</span>
        <AvatarGroup max={3}>
          {ROLES.map((role, i) => (
            <Avatar key={role} name={role} swatch={SWATCH[i]} />
          ))}
        </AvatarGroup>
      </section>
    </div>
  );
}
