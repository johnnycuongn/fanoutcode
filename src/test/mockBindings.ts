// The `vi.mock("@/lib/bindings", () => mockBindings())` factory. `commands.<name>` is a memoised vi.fn per camelCase
// command; `events.<name>.listen` records the callback and `emitForTest(payload)` calls every live listener with a
// Tauri-shaped event. Tests import the typed `commands` and `events` below (the same objects the factory returns), so
// `.mockResolvedValue` type-checks without casting the generated bindings.
import { vi, type Mock } from "vitest";

export type CommandMock = Mock<(...args: never[]) => unknown>;
type Listener = (event: { event: string; id: number; payload: unknown }) => void;
export type EventMock = { listen: Mock<(cb: Listener) => Promise<() => void>>; emitForTest(payload: unknown): void };

const commandFns = new Map<string, CommandMock>();
const eventObjs = new Map<string, EventMock>();

function makeEvent(name: string): EventMock {
  const listeners = new Set<Listener>();
  return {
    listen: vi.fn(async (cb: Listener) => {
      listeners.add(cb);
      return () => {
        listeners.delete(cb);
      };
    }),
    emitForTest(payload) {
      for (const cb of [...listeners]) cb({ event: name, id: 0, payload });
    },
  };
}

export const commands = new Proxy({} as Record<string, CommandMock>, {
  get(_, name) {
    if (typeof name !== "string" || name === "then") return undefined;
    let fn = commandFns.get(name);
    if (!fn) {
      fn = vi.fn() as CommandMock;
      commandFns.set(name, fn);
    }
    return fn;
  },
});

export const events = new Proxy({} as Record<string, EventMock>, {
  get(_, name) {
    if (typeof name !== "string") return undefined;
    let ev = eventObjs.get(name);
    if (!ev) {
      ev = makeEvent(name);
      eventObjs.set(name, ev);
    }
    return ev;
  },
});

export function mockBindings(): { commands: Record<string, CommandMock>; events: Record<string, EventMock> } {
  return { commands, events };
}
