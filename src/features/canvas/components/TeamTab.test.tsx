import "../../../test/reactFlow";
import { describe, it, expect, vi } from "vitest";
import { screen, fireEvent } from "@testing-library/react";
import { UiProvider } from "@/components/ui";
import { renderUi } from "@/test/ui";
import { TASK_ID } from "../fixtures";

const { setViewMode, view } = vi.hoisted(() => {
  const view = { state: null as unknown as Record<string, unknown> };
  return {
    view,
    setViewMode: vi.fn((_: string, m: string) => {
      view.state = { ...view.state, viewMode: m };
    }),
  };
});
vi.mock("../store", async () => {
  const fx = await import("../fixtures");
  const events = fx.sampleBuild();
  const model = (await import("../model")).deriveCanvas({
    steps: fx.samplePlan(),
    events,
    memoryDir: fx.MEMORY_DIR,
    roots: fx.ROOTS,
  });
  const graph = (await import("../graph")).toGraph(model, "HubGraph");
  view.state = {
    taskId: fx.TASK_ID,
    roots: fx.ROOTS,
    events,
    steps: fx.samplePlan(),
    model,
    graph,
    positions: [],
    selection: null,
    replaySeq: null,
    loading: false,
    settings: {
      detail: "Simple",
      showCost: false,
      arrangement: "HubGraph",
      layout: "AutoReflow",
      lens: "ByFile",
      placement: "DockGrows",
    },
    viewMode: "canvas",
    task: { agent_kind: "ClaudeCode", state: { type: "Idle" }, workspace_id: null },
  };
  return {
    useCanvas: () => view.state,
    openCanvas: vi.fn(async () => undefined),
    closeCanvas: vi.fn(),
    setViewMode,
    tidy: vi.fn(async () => undefined),
    startReplay: vi.fn(),
    stopReplay: vi.fn(),
    setReplaySeq: vi.fn(),
    moveNode: vi.fn(),
    select: vi.fn(),
  };
});
const { TeamTab } = await import("./TeamTab");

describe("TeamTab", () => {
  it("toggles between canvas and outline and offers Replay when idle", () => {
    const { rerender } = renderUi(<TeamTab taskId={TASK_ID} />);
    expect(document.querySelector("[data-node-kind]")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Replay" })).toBeTruthy();
    fireEvent.click(screen.getByRole("radio", { name: "Outline" }));
    expect(setViewMode).toHaveBeenCalledWith(TASK_ID, "outline");
    rerender(
      <UiProvider>
        <TeamTab taskId={TASK_ID} />
      </UiProvider>,
    );
    expect(screen.getAllByRole("checkbox")).toHaveLength(6);
  });
  it("shows the hooks empty state for a Codex task", () => {
    const task = view.state.task as Record<string, unknown>;
    view.state = { ...view.state, task: { ...task, agent_kind: "Codex" } };
    renderUi(<TeamTab taskId={TASK_ID} />);
    expect(screen.getByText("Team view needs Claude Code hooks")).toBeTruthy();
  });
});
