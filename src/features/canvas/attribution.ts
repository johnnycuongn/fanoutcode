// Who is on the team: the orchestrator plus one record per subagent, named by its `Step N · role:` dispatch (§6.3).
import type { TaskEvent } from "@/lib/bindings";
import {
  AGENT_TOOLS,
  ORCHESTRATOR_ID,
  agentOf,
  asPostTool,
  asPreTool,
  asStop,
  asSubagentStart,
  asSubagentStop,
} from "./events";

export type AgentRecord = {
  agentId: string;
  role: string;
  adhoc: boolean;
  working: boolean;
  steps: number[];
  description: string | null;
  startedSeq: number;
  stoppedSeq: number | null;
};

const DISPATCH = /^\s*steps?\s+(\d+(?:\s*,\s*\d+)*)\s*[·\-—:]\s*([^:]+):/iu;
const GENERAL = "general-purpose";

/** `Step 5 · frontend: …` → `{ steps: [5], role: "frontend" }`; a hyphen or em dash also separates; else null. */
export function parseDispatch(description: string): { steps: number[]; role: string } | null {
  const m = DISPATCH.exec(description);
  if (!m) return null;
  const role = m[2].trim().toLowerCase();
  if (role === "") return null;
  return { steps: m[1].split(",").map((n) => Number(n.trim())), role };
}

/** True while the main session has a prompt in flight, or its last Stop still lists background tasks. */
export function mainWorking(events: TaskEvent[]): boolean {
  for (let i = events.length - 1; i >= 0; i--) {
    const e = events[i];
    if (e.agent_id !== null) continue;
    switch (e.event.type) {
      case "UserPromptSubmit":
        return true;
      case "Stop":
        return (asStop(e)?.background_tasks ?? 0) > 0;
      case "SessionStart":
      case "StopFailure":
        return false;
      default:
        continue;
    }
  }
  return false;
}

type Dispatch = { preSeq: number; key: string; description: string; type: string | null; started: boolean };

function record(agentId: string, description: string | null, agentType: string, startedSeq: number): AgentRecord {
  const parsed = description ? parseDispatch(description) : null;
  const role = parsed?.role ?? agentType;
  return {
    agentId,
    role,
    adhoc: agentType === GENERAL || (parsed !== null && parsed.role !== agentType),
    working: true,
    steps: parsed?.steps ?? [],
    description,
    startedSeq,
    stoppedSeq: null,
  };
}

/** The orchestrator first, then each subagent in the order it appeared; linear in the number of events. */
export function deriveAgents(events: TaskEvent[]): AgentRecord[] {
  const main: AgentRecord = {
    agentId: ORCHESTRATOR_ID,
    role: "orchestrator",
    adhoc: false,
    working: mainWorking(events),
    steps: [],
    description: null,
    startedSeq: 0,
    stoppedSeq: null,
  };
  const out: AgentRecord[] = [main];
  const byId = new Map<string, AgentRecord>();
  const awaitingStart: Dispatch[] = [];
  const awaitingPost = new Map<string, Dispatch[]>();

  for (const e of events) {
    const pre = asPreTool(e);
    if (pre && AGENT_TOOLS.includes(pre.tool)) {
      const description = typeof pre.input.description === "string" ? pre.input.description : "";
      const type = typeof pre.input.subagent_type === "string" ? pre.input.subagent_type : null;
      const d: Dispatch = { preSeq: e.seq, key: `${agentOf(e)}\u0000${pre.tool}`, description, type, started: false };
      awaitingStart.push(d);
      const queue = awaitingPost.get(d.key);
      if (queue) queue.push(d);
      else awaitingPost.set(d.key, [d]);
      continue;
    }
    const start = asSubagentStart(e);
    if (start) {
      if (byId.has(start.agent_id)) continue;
      let i = start.description === null ? -1 : awaitingStart.findIndex((d) => d.description === start.description);
      if (i < 0 && awaitingStart.length > 0) i = 0;
      const d = i >= 0 ? awaitingStart.splice(i, 1)[0] : null;
      if (d) d.started = true;
      const r = record(start.agent_id, start.description ?? d?.description ?? null, start.agent_type, e.seq);
      byId.set(r.agentId, r);
      out.push(r);
      continue;
    }
    const stop = asSubagentStop(e);
    if (stop) {
      const r = byId.get(stop.agent_id);
      if (r && r.stoppedSeq === null) {
        r.working = false;
        r.stoppedSeq = e.seq;
      }
      continue;
    }
    const post = asPostTool(e);
    if (post && AGENT_TOOLS.includes(post.tool)) {
      const d = awaitingPost.get(`${agentOf(e)}\u0000${post.tool}`)?.shift();
      if (!d || d.started) continue;
      const i = awaitingStart.indexOf(d);
      if (i >= 0) awaitingStart.splice(i, 1);
      const r = record(`dispatch:${d.preSeq}`, d.description || null, d.type ?? GENERAL, d.preSeq);
      r.working = false;
      r.stoppedSeq = e.seq;
      byId.set(r.agentId, r);
      out.push(r);
    }
  }
  return out;
}
