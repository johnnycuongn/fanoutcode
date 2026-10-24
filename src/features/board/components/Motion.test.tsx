import { DndContext } from "@dnd-kit/core";
import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("../ipc");
vi.mock("@/components/ui/EmptyStateLottie", () => ({ default: () => null })); // jsdom has no WebAssembly
vi.mock("./DoneLottie", () => ({ default: () => <div data-testid="lottie" /> }));
import { GLOW_MS } from "@/lib/motion";
import { mockMatchMedia } from "@/test/ui";
import { moveTask } from "../actions";
import { resetBoardStore, upsertTask } from "../store";
import { makeTask } from "../testing/fixtures";
import { mockIpc } from "../testing/mockIpc";
import { Column } from "./Column";

const col = (stage: "Build" | "Done", tasks = [] as ReturnType<typeof makeTask>[]) => (
  <DndContext>
    <Column
      stage={stage}
      lane="*"
      tasks={tasks}
      count={tasks.length}
      readOnly={false}
      compactEmpty
      showProject={false}
      projectsById={{}}
    />
  </DndContext>
);

describe("drop glow", () => {
  beforeEach(() => {
    resetBoardStore();
    mockIpc().ipc.tasksMove.mockResolvedValue(undefined as never);
    vi.useFakeTimers();
  });
  afterEach(() => vi.useRealTimers());

  it("the destination column glows for GLOW_MS after a drop then stops", async () => {
    upsertTask(makeTask({ id: "t-1", stage: "Plan" }));
    render(col("Build"));
    await act(async () => {
      await moveTask("t-1", "Build");
    });
    expect(screen.getByRole("region", { name: "Build" })).toHaveAttribute("data-glow", "true");
    act(() => {
      vi.advanceTimersByTime(GLOW_MS);
    });
    expect(screen.getByRole("region", { name: "Build" })).toHaveAttribute("data-glow", "false");
  });
  it("only the destination glows", async () => {
    upsertTask(makeTask({ id: "t-1", stage: "Plan" }));
    render(col("Done"));
    await act(async () => {
      await moveTask("t-1", "Build");
    });
    expect(screen.getByRole("region", { name: "Done" })).toHaveAttribute("data-glow", "false");
  });
  it("the Done column plays the done moment once when a card lands there", async () => {
    upsertTask(makeTask({ id: "t-1", stage: "Commit" }));
    render(col("Done"));
    expect(screen.queryByTestId("done-moment")).toBeNull();
    await act(async () => {
      await moveTask("t-1", "Done");
    });
    expect(screen.getByTestId("done-moment")).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(GLOW_MS + 1);
    });
    expect(screen.getByTestId("done-moment")).toBeInTheDocument(); // the glow ended; the flourish plays on
    act(() => {
      vi.advanceTimersByTime(1_000);
    });
    expect(screen.queryByTestId("done-moment")).toBeNull();
  });
  it("the done moment does not play under reduced motion", async () => {
    mockMatchMedia(true);
    upsertTask(makeTask({ id: "t-1", stage: "Commit" }));
    render(col("Done"));
    await act(async () => {
      await moveTask("t-1", "Done");
    });
    expect(screen.queryByTestId("done-moment")).toBeNull();
  });
  it("other columns never play the done moment", async () => {
    upsertTask(makeTask({ id: "t-1", stage: "Plan" }));
    render(col("Build"));
    await act(async () => {
      await moveTask("t-1", "Build");
    });
    expect(screen.queryByTestId("done-moment")).toBeNull();
  });
});
