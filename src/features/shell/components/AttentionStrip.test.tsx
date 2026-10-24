// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands } from "@/test/mockBindings";
import { makeTask, makeView, makeCostSummary, needsYou, WORKING } from "@/test/fixtures";
import { tasksStore, projectContainer } from "@/lib/stores/tasksStore";
import { viewsStore, leaf, taskContent } from "@/features/views";
import { settingsStore, type Settings } from "@/features/settings";
import { shellStore, initWindow, setTabOrder } from "../store";
import { AttentionStrip } from "./AttentionStrip";

const container = projectContainer("proj-a1");
const appSettings = (s: Partial<Settings>) => ({ byScope: { app: s as Settings } });
describe("AttentionStrip", () => {
  beforeEach(() => {
    initWindow({ window_id: "w-1", title: "Alpha", container, project_ids: ["proj-a1"] });
    tasksStore.set({
      container,
      loaded: true,
      order: ["task-01", "task-02", "task-03"],
      byId: {
        "task-01": makeTask({
          id: "task-01",
          title: "Older",
          stage: "Plan",
          state: needsYou(),
          updated_at: "2026-10-24T09:00:00Z",
        }),
        "task-02": makeTask({
          id: "task-02",
          title: "Newer",
          stage: "Build",
          state: needsYou(),
          updated_at: "2026-10-24T10:00:00Z",
        }),
        "task-03": makeTask({ id: "task-03", title: "Busy", state: WORKING }),
      },
    });
    viewsStore.set({
      container,
      loaded: true,
      focusedPane: { "view-c3": "pane-aa" },
      pickerFor: null,
      views: [makeView({ id: "view-c3", order: 0, layout: leaf("pane-aa", taskContent("task-03")) })],
    });
    setTabOrder(["view-c3"]);
    shellStore.set({ activeTab: "view-c3" });
    settingsStore.set(appSettings({ cost_overview: false }));
  });
  it("lists needs-you tasks newest first and opens one on click", () => {
    render(<AttentionStrip />);
    const chips = screen.getAllByRole("button");
    expect(chips.map((c) => c.textContent)).toEqual(["Newer · Build done", "Older · Plan written, go?"]);
    fireEvent.click(chips[1]);
    expect(viewsStore.get().views[0].layout).toEqual(leaf("pane-aa", taskContent("task-01")));
  });
  it("collapses on the Board tab", () => {
    shellStore.set({ activeTab: "board" });
    render(<AttentionStrip />);
    expect(screen.queryByRole("toolbar")).toBeNull();
  });
  it("shows the view cost total when the overview is on", async () => {
    settingsStore.set(appSettings({ cost_overview: true }));
    commands.costSummary.mockResolvedValue({
      status: "ok",
      data: makeCostSummary({ cost_usd: 1.25, input_tokens: 900, output_tokens: 120, estimated: true }),
    });
    render(<AttentionStrip />);
    expect(await screen.findByText("~$1.25")).toBeInTheDocument();
  });
});
