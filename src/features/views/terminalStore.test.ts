// @vitest-environment jsdom
import { describe, it, expect, vi, beforeAll, beforeEach } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
vi.mock("@/lib/windowId", () => ({ currentWindowId: () => "w-1" }));
vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: (msg: unknown) => void = () => {};
  },
}));
import { mockBindings, commands, events } from "@/test/mockBindings";
import {
  ACK_FLUSH_BYTES,
  ACK_FLUSH_MS,
  ack,
  attach,
  resize,
  startListening,
  terminalStore,
  toBytes,
} from "./terminalStore";

type FakeChannel = { onmessage(msg: unknown): void };
const session = (over = {}) => ({
  id: "s-1",
  task_id: "t-1",
  kind: "ClaudeCode",
  cwd: "/r",
  driver: "Owned",
  attached_to: null,
  started_at: "2026-10-24T00:00:00Z",
  exited: null,
  ...over,
});
const scrollback = { lines: 1, dropped: 0, bytes: 2, cols: 80, rows: 24 };

describe("terminalStore", () => {
  beforeAll(() => {
    startListening();
  });
  beforeEach(() => {
    vi.clearAllMocks();
    terminalStore.reset();
  });

  it("attach uses the current window id, marks attached and forwards channel bytes", async () => {
    commands.sessionsGet.mockResolvedValue({ status: "ok", data: session() });
    commands.sessionsAttach.mockImplementation(async (...args: never[]) => {
      (args[3] as FakeChannel).onmessage(new Uint8Array([104, 105]).buffer);
      return { status: "ok", data: scrollback };
    });
    const got: Uint8Array[] = [];
    await attach("s-1", "pane-a", (b) => got.push(b));
    expect(commands.sessionsAttach).toHaveBeenCalledWith("s-1", "w-1", "pane-a", expect.anything());
    expect(terminalStore.get()["s-1"]).toMatchObject({
      status: "attached",
      attachedTo: { windowId: "w-1", paneId: "pane-a" },
    });
    expect(Array.from(got[0])).toEqual([104, 105]);
  });

  it("attach to a session held by another pane resolves to elsewhere without attaching", async () => {
    commands.sessionsGet.mockResolvedValue({ status: "ok", data: session({ attached_to: ["w-2", "pane-z"] }) });
    await attach("s-1", "pane-a", () => {});
    expect(terminalStore.get()["s-1"]).toMatchObject({
      status: "elsewhere",
      attachedTo: { windowId: "w-2", paneId: "pane-z" },
    });
    expect(commands.sessionsAttach).not.toHaveBeenCalled();
  });

  it("session-attached for another pane flips this pane to elsewhere", async () => {
    commands.sessionsGet.mockResolvedValue({ status: "ok", data: session() });
    commands.sessionsAttach.mockResolvedValue({ status: "ok", data: scrollback });
    await attach("s-1", "pane-a", () => {});
    events.sessionAttached.emitForTest({ session_id: "s-1", window_id: "w-2", pane_id: "pane-b" });
    expect(terminalStore.get()["s-1"].status).toBe("elsewhere");
    expect(terminalStore.get()["s-1"].broughtAway).toBe(true); // this pane offers "Bring back"
  });

  it("session-exited sets exited with code and clears pending", () => {
    events.sessionQueueChanged.emitForTest({ session_id: "s-1", pending: 3 });
    events.sessionExited.emitForTest({ session_id: "s-1", code: 130 });
    expect(terminalStore.get()["s-1"]).toMatchObject({ status: "exited", exitCode: 130, pending: 0 });
  });

  it("session-queue-changed updates pending", () => {
    events.sessionQueueChanged.emitForTest({ session_id: "s-1", pending: 2 });
    expect(terminalStore.get()["s-1"].pending).toBe(2);
  });

  it("ack coalesces small acks into one call after ACK_FLUSH_MS", () => {
    vi.useFakeTimers();
    ack("s-1", 1024);
    ack("s-1", 1024);
    ack("s-1", 1024);
    expect(commands.sessionsAck).not.toHaveBeenCalled();
    vi.advanceTimersByTime(ACK_FLUSH_MS);
    expect(commands.sessionsAck).toHaveBeenCalledWith("s-1", 3072);
    vi.useRealTimers();
  });

  it("ack flushes immediately at ACK_FLUSH_BYTES", () => {
    ack("s-1", ACK_FLUSH_BYTES);
    expect(commands.sessionsAck).toHaveBeenCalledWith("s-1", ACK_FLUSH_BYTES);
  });

  it("resize skips zero and repeated sizes", () => {
    resize("s-1", 0, 24);
    resize("s-1", 80, 24);
    resize("s-1", 80, 24);
    expect(commands.sessionsResize).toHaveBeenCalledTimes(1);
  });

  it("toBytes accepts ArrayBuffer, Uint8Array and number[]", () => {
    expect(Array.from(toBytes([1, 2]))).toEqual([1, 2]);
    expect(Array.from(toBytes(new Uint8Array([3]).buffer))).toEqual([3]);
    expect(Array.from(toBytes(new Uint8Array([4])))).toEqual([4]);
  });
});
