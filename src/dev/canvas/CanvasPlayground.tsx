// Dev builds only (`#/dev/canvas`): the Team canvas fed from the sample Build one event per 600 ms through the store's
// team-event path, with toggles for arrangement, detail and layout mode. No IPC; the store is seeded directly.
/* eslint-disable fanoutcode/feature-boundaries -- a dev-only harness drives the canvas store and fixtures directly */
import { useEffect, useState, type ReactElement } from "react";
import { Button, Segmented, UiProvider } from "@/components/ui";
import type { Task } from "@/lib/bindings";
import { TeamCanvas } from "@/features/canvas/components/TeamCanvas";
import { MEMORY_DIR, ROOTS, TASK_ID, sampleBuild, samplePlan } from "@/features/canvas/fixtures";
import { getCanvas, pushEvent, seedCanvas, updateSettings, type CanvasSettings } from "@/features/canvas/store";
/* eslint-enable fanoutcode/feature-boundaries */

const NOW = "2026-10-24T05:00:00Z";
const TASK: Task = {
  id: TASK_ID,
  project_id: "p-0a1",
  workspace_id: null,
  title: "Canvas playground",
  stage: "Build",
  state: { type: "Working" },
  priority: "None",
  color: null,
  flagged: false,
  hidden: false,
  archived: false,
  jira_key: null,
  agent_kind: "ClaudeCode",
  worktree: "Off",
  worktree_path: null,
  branch: null,
  memory_dir: MEMORY_DIR,
  primary_session: null,
  claude_session_id: null,
  linked_roots: [],
  base: [],
  created_at: NOW,
  updated_at: NOW,
  stage_entered_at: NOW,
  state_entered_at: NOW,
  links_bound_at: null,
};
const EVENTS = sampleBuild();

function reset(): void {
  seedCanvas(TASK_ID, { task: TASK, steps: samplePlan(), roots: ROOTS });
}

export default function CanvasPlayground(): ReactElement {
  const [next, setNext] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [settings, setSettings] = useState<CanvasSettings>(() => (reset(), getCanvas(TASK_ID).settings));
  const step = () => {
    if (next >= EVENTS.length) return setPlaying(false);
    pushEvent(TASK_ID, EVENTS[next]);
    setNext(next + 1);
  };
  useEffect(() => {
    if (!playing) return;
    const t = setTimeout(step, 600);
    return () => clearTimeout(t);
  });
  const set = (patch: Partial<CanvasSettings>) => {
    updateSettings(TASK_ID, patch);
    setSettings({ ...settings, ...patch });
  };
  return (
    <UiProvider>
      <div className="flex h-screen flex-col bg-page text-ink">
        <div className="flex flex-wrap items-center gap-2 border-b border-line bg-bar px-3 py-2">
          <h1 className="text-sm font-semibold">Canvas playground</h1>
          <Button size="sm" onClick={() => setPlaying(!playing)}>
            {playing ? "Pause" : "Play"}
          </Button>
          <Button size="sm" variant="secondary" onClick={step}>
            Step
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              reset();
              setNext(0);
              setPlaying(false);
            }}
          >
            Reset
          </Button>
          <span className="text-xs text-mute">{`${next} of ${EVENTS.length} events`}</span>
          <Segmented<CanvasSettings["arrangement"]>
            aria-label="Arrangement"
            size="sm"
            value={settings.arrangement}
            onValueChange={(arrangement) => set({ arrangement })}
            options={[
              { value: "HubGraph", label: "Hub graph" },
              { value: "Flow", label: "Flow canvas" },
            ]}
          />
          <Segmented<CanvasSettings["detail"]>
            aria-label="Detail"
            size="sm"
            value={settings.detail}
            onValueChange={(detail) => set({ detail })}
            options={[
              { value: "Simple", label: "Simple" },
              { value: "Medium", label: "Medium" },
              { value: "Detailed", label: "Detailed" },
            ]}
          />
          <Segmented<CanvasSettings["layout"]>
            aria-label="Layout"
            size="sm"
            value={settings.layout}
            onValueChange={(layout) => set({ layout })}
            options={[
              { value: "AutoReflow", label: "Auto re-flow" },
              { value: "Pinned", label: "Pinned" },
            ]}
          />
        </div>
        <div className="min-h-0 flex-1">
          <TeamCanvas taskId={TASK_ID} />
        </div>
      </div>
    </UiProvider>
  );
}
