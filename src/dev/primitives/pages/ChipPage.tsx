import { GitBranch } from "lucide-react";
import { useState, type ReactElement } from "react";
import { Chip, STAGES } from "../../../components/ui";

const HANDBACKS = [
  { status: "needs-you", text: "Spec ready" },
  { status: "needs-you", text: "Plan written, go?" },
  { status: "needs-you", text: "Build done" },
  { status: "needs-you", text: "Message drafted" },
  {
    status: "needs-you",
    text: "Replied: I could not find the test command; which script should I run for the integration suite in this repository?",
  },
  { status: "needs-you", text: "Permission needed" },
  { status: "needs-you", text: "Error" },
  { status: "working", text: "Working" },
  { status: "idle", text: "Idle" },
] as const;

export default function ChipPage(): ReactElement {
  const [filters, setFilters] = useState<string[]>(["needs-you"]);
  const [jira, setJira] = useState(["TC-42", "TC-57"]);
  const current = 2;
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-lg font-semibold">Chip</h1>
      <section className="flex w-80 flex-col gap-2">
        {HANDBACKS.map((h) => (
          <Chip key={h.text} status={h.status}>
            {h.text}
          </Chip>
        ))}
      </section>
      <section className="flex flex-wrap items-center gap-1">
        {STAGES.map((stage, i) => (
          <Chip
            key={stage}
            stage={stage}
            tone={i < current ? "ok" : undefined}
            selected={i === current ? true : undefined}
          >
            {stage}
          </Chip>
        ))}
      </section>
      <section className="flex flex-wrap items-center gap-2">
        {[
          ["needs-you", 3],
          ["working", 2],
          ["idle", 7],
        ].map(([name, count]) => (
          <Chip
            key={name}
            tone="accent"
            count={Number(count)}
            selected={filters.includes(String(name))}
            onClick={() =>
              setFilters((f) => (f.includes(String(name)) ? f.filter((x) => x !== name) : [...f, String(name)]))
            }
          >
            {name}
          </Chip>
        ))}
      </section>
      <section className="flex flex-wrap items-center gap-2">
        {jira.map((key) => (
          <Chip
            key={key}
            icon={<GitBranch className="size-3" />}
            onRemove={() => setJira((j) => j.filter((k) => k !== key))}
            removeLabel={`Remove ${key}`}
          >
            {key}
          </Chip>
        ))}
      </section>
      <section className="flex flex-wrap items-center gap-2">
        {(["rose", "emerald", "indigo", "yellow"] as const).map((swatch) => (
          <Chip key={swatch} swatch={swatch} size="sm" onClick={() => {}}>
            {swatch}
          </Chip>
        ))}
      </section>
    </div>
  );
}
