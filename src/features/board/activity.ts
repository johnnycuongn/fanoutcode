// What a card says the agent is doing: the last line it produced, the file it is editing (with +/-) and its team.
// Seeded from `tasks_activity` at launch, then kept current by `team-event` and `git-changes`.
import type { Container, HookEvent, JsonValue, RepoChanges } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { firstLine } from "./chipText";
import { ipc } from "./ipc";
import { all, track } from "./listen";
import type { CardActivity, TeamMember } from "./types";

/** The line a role prints when it finishes a stage (overview: `SENTINEL_PREFIX`). */
const SENTINEL_PREFIX = "TEAMCODE: ";
const FILE_TOOLS: Record<string, string> = {
  Edit: "file_path",
  Write: "file_path",
  MultiEdit: "file_path",
  NotebookEdit: "notebook_path",
};

export const emptyActivity: CardActivity = { lastLine: null, currentFile: null, team: [] };

function field(input: JsonValue, key: string): string | null {
  if (typeof input !== "object" || input === null || Array.isArray(input)) return null;
  const value = input[key];
  return typeof value === "string" ? value : null;
}

const basename = (path: string): string => path.slice(path.lastIndexOf("/") + 1);

function withLine(prev: CardActivity, lastLine: string): CardActivity {
  return prev.lastLine === lastLine ? prev : { ...prev, lastLine };
}

function startSubagent(team: TeamMember[], agentId: string, role: string): TeamMember[] {
  const seeded = team.findIndex((m) => m.agentId === null && m.role === role);
  const member: TeamMember = { agentId, role, working: true };
  if (seeded >= 0) return team.map((m, i) => (i === seeded ? member : m));
  return [...team.filter((m) => m.agentId !== agentId), member];
}

function reduceToolUse(prev: CardActivity, tool: string, input: JsonValue): CardActivity {
  const fileKey = FILE_TOOLS[tool];
  const path = fileKey ? field(input, fileKey) : null;
  if (path) {
    const same = prev.currentFile?.path === path;
    return {
      ...prev,
      currentFile: same ? prev.currentFile : { path, added: 0, removed: 0 },
      lastLine: `Editing ${basename(path)}`,
    };
  }
  const command = tool === "Bash" ? field(input, "command") : null;
  return command ? withLine(prev, `$ ${firstLine(command, 80)}`) : prev;
}

/** Folds one hook event into a card's activity; an event the card does not show returns `prev` itself. */
export function reduceActivity(prev: CardActivity, event: HookEvent): CardActivity {
  switch (event.type) {
    case "PostToolUse":
      return reduceToolUse(prev, event.tool, event.input);
    case "Stop": {
      const line = event.last_assistant_message
        .split(/\r?\n/)
        .map((l) => l.trim())
        .find((l) => l && !l.startsWith(SENTINEL_PREFIX));
      return line ? withLine(prev, firstLine(line)) : prev;
    }
    case "SubagentStart":
      return { ...prev, team: startSubagent(prev.team, event.agent_id, event.agent_type) };
    case "SubagentStop":
      return { ...prev, team: prev.team.map((m) => (m.agentId === event.agent_id ? { ...m, working: false } : m)) };
    case "SessionStart":
      return { ...prev, team: [], lastLine: "Session started" };
    case "PermissionRequest":
      return withLine(prev, `Permission needed: ${event.tool}`);
    case "Notification":
      return withLine(prev, firstLine(event.message));
    case "StopFailure":
      return withLine(prev, `Error: ${firstLine(event.message)}`);
    default:
      return prev;
  }
}

export const activityStore = createStore<Record<string, CardActivity>>({});

export const useActivity = (taskId: string): CardActivity =>
  activityStore.useSelector((s) => s[taskId] ?? emptyActivity);

/** Seeds cards from the launch snapshot; a task that already has live activity keeps it. */
export async function loadActivity(container: Container): Promise<void> {
  const rows = await ipc.tasksActivity(container);
  activityStore.set((s) => {
    const next = { ...s };
    for (const row of rows) {
      if (next[row.task_id]) continue;
      next[row.task_id] = {
        lastLine: null,
        currentFile: row.current_file ? { path: row.current_file, added: row.plus, removed: row.minus } : null,
        team: row.team.map((t) => ({ agentId: null, role: t.name, working: t.state.type === "Working" })),
      };
    }
    return next;
  });
}

/** Fills +/- for the file the agent is editing; the snapshot's path may be repo-relative. */
export function applyChanges(taskId: string, changes: RepoChanges[]): void {
  const current = activityStore.get()[taskId]?.currentFile;
  if (!current) return;
  for (const repo of changes) {
    const file = repo.files.find((f) => f.path === current.path || `${repo.repo_root}/${f.path}` === current.path);
    if (!file) continue;
    if (file.added === current.added && file.removed === current.removed) return;
    activityStore.set((s) => ({
      ...s,
      [taskId]: { ...s[taskId]!, currentFile: { ...current, added: file.added, removed: file.removed } },
    }));
    return;
  }
}

export function startActivityEvents(): () => void {
  return all(
    track(
      ipc.on("team-event", ({ task_id, event }) => {
        const prev = activityStore.get()[task_id] ?? emptyActivity;
        const next = reduceActivity(prev, event.event);
        if (next !== prev || !activityStore.get()[task_id]) activityStore.set((s) => ({ ...s, [task_id]: next }));
      }),
    ),
    track(
      ipc.on("git-changes", ({ task_id }) => {
        if (!activityStore.get()[task_id]?.currentFile) return;
        ipc.gitChanges(task_id).then(
          (changes) => applyChanges(task_id, changes),
          () => {}, // the card keeps its last numbers
        );
      }),
    ),
  );
}
