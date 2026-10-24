import { Inbox } from "lucide-react";
import type { ReactElement } from "react";
import { Button, EmptyState } from "../../../components/ui";
import art from "../assets/empty-column.lottie?url";

export default function EmptyStatePage(): ReactElement {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">EmptyState</h1>
      <section className="flex flex-wrap items-start gap-6">
        {(["sm", "md", "lg"] as const).map((size) => (
          <div key={size} className="w-64 rounded-well border border-line bg-well">
            <EmptyState size={size} animation={art} title={`Size ${size}`} description="Placeholder animation." />
          </div>
        ))}
        <div className="w-64 rounded-well border border-line bg-well">
          <EmptyState icon={<Inbox className="size-8" />} title="No tasks in Build" description="Drag a card here." />
        </div>
        <div className="w-64 rounded-well border border-line bg-well">
          <EmptyState
            animation={art}
            title="Nothing needs you"
            description="Agents are working or idle."
            action={<Button variant="primary">New task</Button>}
          />
        </div>
      </section>
    </div>
  );
}
