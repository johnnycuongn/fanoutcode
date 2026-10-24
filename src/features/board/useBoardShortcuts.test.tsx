import { fireEvent, render } from "@testing-library/react";
import { useRef } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("./ipc");
import { boardStore, resetBoardStore, setFocusedTask, upsertTask } from "./store";
import { makeTask } from "./testing/fixtures";
import { mockIpc } from "./testing/mockIpc";
import { useBoardShortcuts } from "./useBoardShortcuts";

function Host({ confirm }: { confirm: () => Promise<boolean> }) {
  const ref = useRef<HTMLDivElement>(null);
  useBoardShortcuts(ref, { confirm });
  return <div ref={ref} tabIndex={0} data-testid="root" />;
}

describe("useBoardShortcuts", () => {
  beforeEach(() => resetBoardStore());
  it("⌘E starts renaming the focused card and ⌘⇧A archives it", async () => {
    const m = mockIpc();
    upsertTask(makeTask({ id: "t-1" }));
    setFocusedTask("t-1");
    const { getByTestId } = render(<Host confirm={async () => true} />);
    fireEvent.keyDown(getByTestId("root"), { key: "e", metaKey: true });
    expect(boardStore.get().renamingTaskId).toBe("t-1");
    fireEvent.keyDown(getByTestId("root"), { key: "a", metaKey: true, shiftKey: true });
    await Promise.resolve();
    expect(m.ipc.tasksArchive).toHaveBeenCalledWith("t-1");
  });
  it("⌘⇧A restores an archived card", async () => {
    const m = mockIpc();
    upsertTask(makeTask({ id: "t-1", archived: true }));
    setFocusedTask("t-1");
    const { getByTestId } = render(<Host confirm={async () => true} />);
    fireEvent.keyDown(getByTestId("root"), { key: "A", metaKey: true, shiftKey: true });
    expect(m.ipc.tasksRestore).toHaveBeenCalledWith("t-1");
  });
  it("does nothing without a focused card and leaves other keys alone", () => {
    const m = mockIpc();
    const { getByTestId } = render(<Host confirm={async () => true} />);
    fireEvent.keyDown(getByTestId("root"), { key: "a", metaKey: true, shiftKey: true });
    expect(m.ipc.tasksArchive).not.toHaveBeenCalled();
    upsertTask(makeTask({ id: "t-1" }));
    setFocusedTask("t-1");
    const notHandled = fireEvent.keyDown(getByTestId("root"), { key: "k", metaKey: true });
    expect(notHandled).toBe(true); // not default-prevented, so the window-level map still sees it
  });
});
