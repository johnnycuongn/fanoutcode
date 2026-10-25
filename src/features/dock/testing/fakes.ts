// An in-memory DockPersist for tests.
import type { DockPersist, DockState } from "../types";

export const mem = (): DockPersist & { last: DockState | null } => {
  const p = {
    last: null as DockState | null,
    read: () => p.last,
    write: (s: DockState) => {
      p.last = s;
    },
  };
  return p;
};
