import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/react";
import type { Task } from "@/lib/bindings";
import { TASK_ID } from "./fixtures";

const { splitFocusedPane } = vi.hoisted(() => ({ splitFocusedPane: vi.fn(async () => undefined) }));
vi.mock("@/features/views", () => ({ splitFocusedPane }));
vi.mock("./components/TeamTab", () => ({
  TeamTab: ({ taskId, variant }: { taskId: string; variant: string }) => (
    <div data-team-tab={taskId} data-variant={variant} />
  ),
}));
const { renderTeam, onOpenTeamPane, teamCanvasContent } = await import("./teamPage");
const task = { id: TASK_ID } as Task;

describe("task-page wiring", () => {
  it("renderTeam mounts the dock variant of the Team tab for the task", () => {
    const { container } = render(<>{renderTeam(task)}</>);
    const tab = container.querySelector("[data-team-tab]")!;
    expect(tab.getAttribute("data-team-tab")).toBe(TASK_ID);
    expect(tab.getAttribute("data-variant")).toBe("dock");
  });
  it("onOpenTeamPane splits the focused pane side by side with the task's canvas", async () => {
    onOpenTeamPane(task);
    await vi.waitFor(() => expect(splitFocusedPane).toHaveBeenCalledWith("Horizontal", teamCanvasContent(TASK_ID)));
    expect(teamCanvasContent(TASK_ID)).toEqual({ type: "TeamCanvas", task_id: TASK_ID });
  });
});
