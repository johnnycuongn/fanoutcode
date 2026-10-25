// The drill-in panel (§10.7): opened by clicking an agent or a file; By file or Event stream by settings, with
// filter chips by kind and by agent that apply to both lenses.
import { X } from "lucide-react";
import { useMemo, useState, type ReactElement } from "react";
import { Badge, Chip, IconButton, Panel, PanelBody, PanelHeader } from "@/components/ui";
import { byFileLens, eventStreamLens, streamAgents, type ByFileGroup, type RollUp, type StreamFilter } from "../lens";
import { eventsUntil } from "../replay";
import { select, useCanvas } from "../store";
import type { StreamKind } from "../types";
import { shortPath } from "./cards/cardParts";
import { ByFileLens } from "./ByFileLens";
import { EventStreamLens } from "./EventStreamLens";

const KINDS: StreamKind[] = ["edit", "command", "read", "agent", "hook"];

function toggle<T>(set: ReadonlySet<T>, v: T): ReadonlySet<T> {
  const next = new Set(set);
  if (!next.delete(v)) next.add(v);
  return next;
}

function filterByFile(lens: { groups: ByFileGroup[]; rollUp: RollUp }, f: StreamFilter) {
  const kind = (k: StreamKind) => f.kinds.size === 0 || f.kinds.has(k);
  const agent = (a: string) => f.agents.size === 0 || f.agents.has(a);
  return {
    groups: kind("edit")
      ? lens.groups.map((g) => ({ ...g, edits: g.edits.filter((e) => agent(e.agentId)) })).filter((g) => g.edits.length)
      : [],
    rollUp: {
      commands: kind("command") ? lens.rollUp.commands.filter((c) => agent(c.agentId)) : [],
      hooks: kind("hook") ? lens.rollUp.hooks.filter((h) => agent(h.agentId)) : [],
    },
  };
}

export function DrillIn({ taskId }: { taskId: string }): ReactElement | null {
  const s = useCanvas(taskId);
  const [filter, setFilter] = useState<StreamFilter>({ kinds: new Set(), agents: new Set() });
  const events = useMemo(
    () => (s.replaySeq === null ? s.events : eventsUntil(s.events, s.replaySeq)),
    [s.events, s.replaySeq],
  );
  const target = s.selection;
  if (!target) return null;
  const roles = Object.fromEntries(s.model.agents.map((a) => [a.agentId, a.role]));
  const agents = streamAgents(s.model, target);
  const name = target.kind === "agent" ? (roles[agents[0]] ?? agents[0]) : shortPath(target.id.replace(/^file:/, ""));
  const eventStream = s.settings.lens === "EventStream";
  return (
    <Panel
      as="aside"
      variant="card"
      padding="none"
      aria-label={`Drill-in: ${name}`}
      className="absolute top-2 right-2 bottom-2 z-10 flex w-[360px] flex-col overflow-hidden"
    >
      <PanelHeader
        title={name}
        actions={
          <>
            <Badge size="sm">{eventStream ? "Event stream" : "By file"}</Badge>
            <IconButton size="sm" label="Close" icon={<X size={14} />} onClick={() => select(taskId, null)} />
          </>
        }
      />
      <div className="flex flex-wrap gap-1 border-b border-line px-3 py-2">
        {KINDS.map((k) => (
          <Chip
            key={k}
            size="sm"
            selected={filter.kinds.has(k)}
            onClick={() => setFilter({ ...filter, kinds: toggle(filter.kinds, k) })}
          >
            {k}
          </Chip>
        ))}
        {agents.length > 1 &&
          agents.map((a) => (
            <Chip
              key={a}
              size="sm"
              tone="accent"
              selected={filter.agents.has(a)}
              onClick={() => setFilter({ ...filter, agents: toggle(filter.agents, a) })}
            >
              {roles[a] ?? a}
            </Chip>
          ))}
      </div>
      <PanelBody scroll className="min-h-0 flex-1">
        {eventStream ? (
          <EventStreamLens rows={eventStreamLens(s.model, events, target, filter)} />
        ) : (
          <ByFileLens {...filterByFile(byFileLens(s.model, events, target), filter)} roles={roles} />
        )}
      </PanelBody>
    </Panel>
  );
}
