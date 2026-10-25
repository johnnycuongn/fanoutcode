// Each session's terminal attachment as this window sees it (Plan 04): attaching, attached here, attached in
// another pane ("elsewhere"), exited; the prompts waiting for the agent to go idle; and the coalesced acks that
// drive the backend's flow control. The window id always comes from `currentWindowId()`, never from a caller.
import { Channel } from "@tauri-apps/api/core";
import { commands, events, type Result, type Scrollback } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { currentWindowId } from "@/lib/windowId";

export type TerminalStatus = "detached" | "attaching" | "attached" | "elsewhere" | "exited";
export type TerminalState = {
  status: TerminalStatus;
  attachedTo?: { windowId: string; paneId: string };
  exitCode?: number;
  pending: number;
  scrollback?: Scrollback;
};

/** An ack is sent once this many bytes are drawn, or `ACK_FLUSH_MS` after the first undrawn ack. */
export const ACK_FLUSH_BYTES = 65_536;
export const ACK_FLUSH_MS = 50;

const DETACHED: TerminalState = { status: "detached", pending: 0 };

export const terminalStore = createStore<Record<string, TerminalState>>({});

/** The pane of this window that last attached each session. */
const localPane = new Map<string, string>();
const lastSize = new Map<string, string>();
const acks = new Map<string, { bytes: number; timer?: ReturnType<typeof setTimeout> }>();

function patch(sessionId: string, next: Partial<TerminalState>): void {
  terminalStore.set((s) => ({ ...s, [sessionId]: { ...(s[sessionId] ?? DETACHED), ...next } }));
}

function unwrap<T>(r: Result<T, string>): T {
  if (r.status === "error") throw new Error(r.error);
  return r.data;
}

/** A channel message as bytes: raw (`ArrayBuffer` / `Uint8Array`) or a JSON `number[]`. */
export function toBytes(msg: unknown): Uint8Array {
  if (msg instanceof Uint8Array) return msg;
  if (msg instanceof ArrayBuffer) return new Uint8Array(msg);
  if (Array.isArray(msg)) return Uint8Array.from(msg as number[]);
  return new Uint8Array(0);
}

function channelFor(onData: (bytes: Uint8Array) => void): Channel<unknown> {
  const channel = new Channel<unknown>();
  channel.onmessage = (msg) => onData(toBytes(msg));
  return channel;
}

/** `attached-elsewhere:<window>:<pane>` from the backend, as the holder. */
function holderFrom(err: string): { windowId: string; paneId: string } | null {
  const m = /^attached-elsewhere:([^:]+):(.+)$/.exec(err);
  return m ? { windowId: m[1], paneId: m[2] } : null;
}

async function connect(
  sessionId: string,
  paneId: string,
  onData: (bytes: Uint8Array) => void,
  call: typeof commands.sessionsAttach,
  exited: number | null,
): Promise<void> {
  const windowId = currentWindowId();
  const r = await call(sessionId, windowId, paneId, channelFor(onData));
  if (r.status === "error") {
    const holder = holderFrom(r.error);
    if (holder) return patch(sessionId, { status: "elsewhere", attachedTo: holder });
    patch(sessionId, { status: "detached" });
    throw new Error(r.error);
  }
  patch(sessionId, {
    status: exited === null ? "attached" : "exited",
    attachedTo: { windowId, paneId },
    scrollback: r.data,
    ...(exited === null ? {} : { exitCode: exited }),
  });
}

/** Attaches `paneId` of this window: the scrollback replays through `onData`, then live output follows. A session
 *  another pane holds resolves to `elsewhere` without attaching. */
export async function attach(sessionId: string, paneId: string, onData: (bytes: Uint8Array) => void): Promise<void> {
  const windowId = currentWindowId();
  localPane.set(sessionId, paneId);
  patch(sessionId, { status: "attaching" });
  let session;
  try {
    session = unwrap(await commands.sessionsGet(sessionId));
  } catch (err) {
    patch(sessionId, { status: "detached" });
    throw err;
  }
  const holder = session.attached_to;
  if (holder && (holder[0] !== windowId || holder[1] !== paneId)) {
    return patch(sessionId, { status: "elsewhere", attachedTo: { windowId: holder[0], paneId: holder[1] } });
  }
  await connect(sessionId, paneId, onData, commands.sessionsAttach, session.exited);
}

/** Moves the session to `paneId` of this window from wherever it is attached ("Bring here"). */
export async function bring(sessionId: string, paneId: string, onData: (bytes: Uint8Array) => void): Promise<void> {
  localPane.set(sessionId, paneId);
  patch(sessionId, { status: "attaching" });
  await connect(sessionId, paneId, onData, commands.sessionsBring, null);
}

/** The pane let go (unmount). A session another pane holds keeps that state. */
export async function detach(sessionId: string): Promise<void> {
  flushAck(sessionId);
  localPane.delete(sessionId);
  lastSize.delete(sessionId);
  const current = terminalStore.get()[sessionId];
  if (current?.status === "elsewhere") return;
  if (current && current.status !== "exited") patch(sessionId, { status: "detached", attachedTo: undefined });
  await commands.sessionsDetach(sessionId);
}

const encoder = new TextEncoder();

/** Keystrokes and pastes; strings are sent as UTF-8. */
export function write(sessionId: string, data: string | Uint8Array): void {
  const bytes = typeof data === "string" ? encoder.encode(data) : data;
  if (bytes.length === 0) return;
  void commands.sessionsWrite(sessionId, Array.from(bytes));
}

/** Skips a zero dimension (a collapsed pane) and a size equal to the last one sent. */
export function resize(sessionId: string, cols: number, rows: number): void {
  if (cols <= 0 || rows <= 0) return;
  const key = `${cols}x${rows}`;
  if (lastSize.get(sessionId) === key) return;
  lastSize.set(sessionId, key);
  void commands.sessionsResize(sessionId, cols, rows);
}

function flushAck(sessionId: string): void {
  const a = acks.get(sessionId);
  if (!a) return;
  if (a.timer) clearTimeout(a.timer);
  acks.delete(sessionId);
  if (a.bytes > 0) void commands.sessionsAck(sessionId, a.bytes);
}

/** `bytes` more were drawn; coalesced into one `sessions_ack` per `ACK_FLUSH_BYTES` or `ACK_FLUSH_MS`. */
export function ack(sessionId: string, bytes: number): void {
  const a = acks.get(sessionId) ?? { bytes: 0 };
  a.bytes += bytes;
  acks.set(sessionId, a);
  if (a.bytes >= ACK_FLUSH_BYTES) return flushAck(sessionId);
  a.timer ??= setTimeout(() => flushAck(sessionId), ACK_FLUSH_MS);
}

let listening: Promise<Array<() => void>> | null = null;

/** Subscribes to `session-attached`, `session-exited` and `session-queue-changed` once; returns the unsubscribe. */
export function startListening(): () => void {
  listening ??= Promise.all([
    events.sessionAttached.listen(({ payload }) => {
      const here = payload.window_id === currentWindowId() && localPane.get(payload.session_id) === payload.pane_id;
      const attachedTo = { windowId: payload.window_id, paneId: payload.pane_id };
      const current = terminalStore.get()[payload.session_id];
      if (here) patch(payload.session_id, { status: current?.status === "exited" ? "exited" : "attached", attachedTo });
      else if (current) patch(payload.session_id, { status: "elsewhere", attachedTo });
    }),
    events.sessionExited.listen(({ payload }) => {
      patch(payload.session_id, { status: "exited", exitCode: payload.code ?? undefined, pending: 0 });
    }),
    events.sessionQueueChanged.listen(({ payload }) => {
      patch(payload.session_id, { pending: payload.pending });
    }),
  ]);
  const started = listening;
  return () => {
    void started.then((fns) => fns.forEach((f) => f()));
    if (listening === started) listening = null;
  };
}

/** This session's state; `{ status: "detached", pending: 0 }` until something happens. */
export function useTerminal(sessionId: string): TerminalState {
  return terminalStore.useSelector((s) => s[sessionId] ?? DETACHED);
}
