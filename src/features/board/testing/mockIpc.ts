// Test helper for `vi.mock("./ipc")`. Resets every mocked ipc function, gives the list-style ones an empty default,
// and makes `ipc.on` register callbacks synchronously so `emit` reaches them.
import { vi, type Mock, type Mocked } from "vitest";
import { ipc } from "../ipc";
import { makeCost } from "./fixtures";

export function mockIpc(): { ipc: Mocked<typeof ipc>; emit(name: string, payload: unknown): void; unlisten: Mock } {
  const listeners = new Map<string, Set<(payload: unknown) => void>>();
  const unlisten = vi.fn();
  for (const fn of Object.values(ipc)) (fn as unknown as Mock).mockReset();
  const mocked = ipc as Mocked<typeof ipc>;
  mocked.tasksList.mockResolvedValue([]);
  mocked.windowsList.mockResolvedValue([]);
  mocked.tasksActivity.mockResolvedValue([]);
  mocked.gitChanges.mockResolvedValue([]);
  mocked.costSummary.mockResolvedValue(makeCost());
  (mocked.on as unknown as Mock).mockImplementation((name: string, cb: (payload: unknown) => void) => {
    const set = listeners.get(name) ?? new Set();
    listeners.set(name, set);
    set.add(cb);
    return Promise.resolve(() => {
      set.delete(cb);
      unlisten();
    });
  });
  return {
    ipc: mocked,
    unlisten,
    emit(name, payload) {
      for (const cb of [...(listeners.get(name) ?? [])]) cb(payload);
    },
  };
}
