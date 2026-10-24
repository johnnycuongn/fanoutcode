import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("../ipc");
vi.mock("@/components/ui/EmptyStateLottie", () => ({ default: () => null })); // jsdom has no WebAssembly
import { resetBoardStore } from "../store";
import { makeProject, makeTask, projectContainer } from "../testing/fixtures";
import { mockIpc } from "../testing/mockIpc";
import { mountUi } from "../testing/render";
import { Board } from "./Board";

const STAGE_ORDER = ["Spec", "Plan", "Build", "Review", "Test", "Commit", "Done"];
const settings = {
  detailLevel: "Simple",
  detailLevelByProject: {},
  costOverview: false,
  workspaceBoard: "Swimlanes",
  projectLayer: "MultiRoot",
} as const;
const props = {
  container: projectContainer("p-1"),
  projects: [makeProject({ id: "p-1" })],
  settings,
  focusedProject: null,
  currentWindowId: "w-1",
  onOpenTask: vi.fn(),
  onNewTask: vi.fn(),
};

// jsdom lays nothing out, so give each column a rectangle of its own: dnd-kit decides the target from them.
function layOutColumns() {
  return vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
    // The drag overlay's copy of a card sits outside the columns: place it where the original is.
    const section = this.closest("[data-stage]") ?? document.querySelector("[data-stage]:has(article)");
    const index = section ? STAGE_ORDER.indexOf(section.getAttribute("data-stage")!) : 0;
    const isCard = this.tagName === "ARTICLE";
    const rect = { left: index * 200, top: isCard ? 40 : 0, width: isCard ? 180 : 190, height: isCard ? 60 : 600 };
    return {
      ...rect,
      x: rect.left,
      y: rect.top,
      right: rect.left + rect.width,
      bottom: rect.top + rect.height,
      toJSON: () => ({}),
    };
  });
}

describe("keyboard drag", () => {
  let m: ReturnType<typeof mockIpc>;
  let rects: ReturnType<typeof layOutColumns>;
  beforeEach(() => {
    resetBoardStore();
    m = mockIpc();
    m.ipc.tasksList.mockResolvedValue([makeTask({ id: "a", title: "Drag me", stage: "Plan" })]);
    rects = layOutColumns();
  });
  afterEach(() => rects.mockRestore());

  it("Space picks a card up, ArrowRight steps to the next column and Space drops it through tasks_move", async () => {
    const user = userEvent.setup();
    mountUi(<Board {...props} />);
    const card = await screen.findByRole("article");
    act(() => card.focus());
    await user.keyboard(" ");
    await user.keyboard("{ArrowRight}");
    const live = () => document.querySelector("[role=status]")?.textContent;
    await waitFor(() => expect(live()).toBe("Drag me is over Build"));
    await user.keyboard(" ");
    await waitFor(() => expect(m.ipc.tasksMove).toHaveBeenCalledWith("a", "Build"));
  });
  it("Enter on a card opens it instead of starting a drag", async () => {
    const user = userEvent.setup();
    const onOpenTask = vi.fn();
    mountUi(<Board {...props} onOpenTask={onOpenTask} />);
    const card = await screen.findByRole("article");
    act(() => card.focus());
    await user.keyboard("{Enter}");
    expect(onOpenTask).toHaveBeenCalledWith("a", { kind: "current" });
    expect(m.ipc.tasksMove).not.toHaveBeenCalled();
  });
  it("Escape cancels a keyboard drag without moving the card", async () => {
    const user = userEvent.setup();
    mountUi(<Board {...props} />);
    const card = await screen.findByRole("article");
    act(() => card.focus());
    await user.keyboard(" ");
    await user.keyboard("{ArrowRight}");
    await user.keyboard("{Escape}");
    await user.keyboard(" ");
    expect(m.ipc.tasksMove).not.toHaveBeenCalled();
  });
});
