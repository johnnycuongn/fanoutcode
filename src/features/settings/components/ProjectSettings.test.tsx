// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands } from "@/test/mockBindings";
import { makeProject, makeTask, makeCostSummary } from "@/test/fixtures";
import { projectsStore } from "@/lib/stores/projectsStore";
import { tasksStore } from "@/lib/stores/tasksStore";
import { settingsStore, projectScope } from "../store";
import type { Settings } from "../types";
import { ProjectSettings } from "./ProjectSettings";

const scope = projectScope("proj-a1");
const project = {
  worktree: "Off",
  worktree_convention: null,
  worktrees_dir: "/r/alpha/.worktrees/",
  test_command: "",
  test_enabled: true,
  advance_on_green: false,
  may_commit: false,
  agent_kind: "ClaudeCode",
  detail_level: null,
  cost_overview: true,
} as unknown as Settings;
describe("ProjectSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    projectsStore.set({ byId: { "proj-a1": makeProject({ id: "proj-a1", name: "Alpha" }) }, order: ["proj-a1"] });
    settingsStore.set({
      byScope: { app: { cost_overview: true } as Settings, "project:proj-a1": project },
      loading: {},
    });
    commands.settingsSet.mockResolvedValue({ status: "ok", data: null });
    tasksStore.set({ byId: { "task-b2": makeTask({ id: "task-b2", title: "Login" }) }, order: ["task-b2"] });
    commands.costSummary.mockResolvedValue({
      status: "ok",
      data: makeCostSummary({
        cost_usd: 4.5,
        input_tokens: 1200,
        output_tokens: 300,
        by_task: [{ key: "task-b2", cost_usd: 4.5, input_tokens: 1200, output_tokens: 300, estimated: false }],
      }),
    });
  });
  it("picks a memory folder through the native picker", async () => {
    commands.memoryPickLocation.mockResolvedValue({ status: "ok", data: "/r/alpha/docs/tasks" });
    commands.projectsUpdate.mockResolvedValue({
      status: "ok",
      data: makeProject({ id: "proj-a1", memory_location: { type: "Folder", path: "/r/alpha/docs/tasks" } }),
    });
    render(<ProjectSettings projectId="proj-a1" />);
    expect(screen.getByText("App storage")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Choose folder…" }));
    await vi.waitFor(() =>
      expect(commands.projectsUpdate).toHaveBeenCalledWith(
        "proj-a1",
        expect.objectContaining({ memory_location: { type: "Folder", path: "/r/alpha/docs/tasks" } }),
      ),
    );
    expect(commands.settingsSet).not.toHaveBeenCalledWith(scope, "memory_location", expect.anything());
  });
  it('keeps "may commit" off by default and writes it when switched', () => {
    render(<ProjectSettings projectId="proj-a1" />);
    const sw = screen.getByRole("switch", { name: "Fanout Code may commit" });
    expect(sw).not.toBeChecked();
    fireEvent.click(sw);
    expect(commands.settingsSet).toHaveBeenCalledWith(scope, "may_commit", true);
  });
  it("disables advance-on-green without a test command and shows the worktrees dir only for Fanout Code-managed", () => {
    render(<ProjectSettings projectId="proj-a1" />);
    expect(screen.getByRole("switch", { name: "Advance on green" })).toBeDisabled();
    expect(screen.queryByLabelText("Worktrees dir")).toBeNull();
    fireEvent.click(screen.getByRole("radio", { name: "Fanout Code-managed" }));
    expect(commands.settingsSet).toHaveBeenCalledWith(scope, "worktree_convention", "FanoutCodeManaged");
  });
  it("shows the costs panel when the overview is on", async () => {
    render(<ProjectSettings projectId="proj-a1" />);
    expect(await screen.findAllByText("$4.50")).toHaveLength(2);
    expect(screen.getByText("Login")).toBeInTheDocument();
    expect(screen.getByText("1200 in · 300 out")).toBeInTheDocument();
  });
  it("lays rows out against its own width, not the window's", () => {
    const { container, rerender } = render(<ProjectSettings projectId="proj-a1" />);
    expect(container.firstChild).toHaveClass("@container", "min-w-0", "gap-5");
    expect(screen.getByRole("region", { name: "Memory" })).toHaveClass("min-w-0");
    expect(screen.getByText("Name").closest("div")).toHaveClass("grid-cols-1", "@md:grid-cols-[minmax(8rem,1fr)_2fr]");
    rerender(<ProjectSettings projectId="proj-a1" compact />);
    expect(container.firstChild).toHaveClass("gap-4");
  });
  it("lets the memory location wrap its buttons and truncate a long folder path", () => {
    projectsStore.set({
      byId: {
        "proj-a1": makeProject({ id: "proj-a1", memory_location: { type: "Folder", path: "/r/alpha/docs/tasks" } }),
      },
      order: ["proj-a1"],
    });
    render(<ProjectSettings projectId="proj-a1" compact />);
    expect(screen.getByText("/r/alpha/docs/tasks")).toHaveClass("truncate");
    expect(screen.getByRole("button", { name: "Choose folder…" }).parentElement).toHaveClass("flex-wrap");
    expect(screen.getByRole("button", { name: "Use app storage" })).toBeInTheDocument();
  });
});
