// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { act, createRef } from "react";
import { render, screen, waitFor } from "@testing-library/react";

const fakeTerm = vi.hoisted(() => ({
  cols: 80,
  rows: 24,
  open: vi.fn(),
  loadAddon: vi.fn(),
  write: vi.fn((_data: Uint8Array, cb?: () => void) => cb?.()),
  onData: vi.fn(() => ({ dispose: vi.fn() })),
  onBinary: vi.fn(() => ({ dispose: vi.fn() })),
  dispose: vi.fn(),
  focus: vi.fn(),
  attachCustomKeyEventHandler: vi.fn(),
  options: {} as Record<string, unknown>,
}));
const settings = vi.hoisted(() => ({ current: undefined as unknown }));

vi.mock("@/lib/bindings", () => mockBindings());
vi.mock("@xterm/xterm", () => ({
  Terminal: vi.fn(function Terminal() {
    return fakeTerm;
  }),
}));
vi.mock("@xterm/addon-fit", () => ({
  FitAddon: vi.fn(function FitAddon() {
    return { fit: vi.fn(), dispose: vi.fn() };
  }),
}));
vi.mock("@xterm/addon-webgl", () => ({
  WebglAddon: vi.fn(function WebglAddon() {
    return { onContextLoss: vi.fn(), dispose: vi.fn() };
  }),
}));
vi.mock("@xterm/addon-canvas", () => ({
  CanvasAddon: vi.fn(function CanvasAddon() {
    return { dispose: vi.fn() };
  }),
}));
vi.mock("@/features/settings", () => ({ useSettings: () => settings.current }));
vi.mock("../terminalStore", async (orig) => ({
  ...(await orig<typeof import("../terminalStore")>()),
  attach: vi.fn().mockResolvedValue(undefined),
  bring: vi.fn().mockResolvedValue(undefined),
  detach: vi.fn(),
  ack: vi.fn(),
  resize: vi.fn(),
  write: vi.fn(),
}));
import { mockBindings } from "@/test/mockBindings";
import { attach, ack, detach, terminalStore } from "../terminalStore";
import { TerminalPane, type TerminalPaneHandle } from "./TerminalPane";

const attachMock = vi.mocked(attach);

describe("TerminalPane", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    terminalStore.reset();
    settings.current = undefined;
  });

  it("attaches on mount with the pane identity and detaches on unmount", async () => {
    const { unmount } = render(<TerminalPane sessionId="s-1" paneId="pane-a" focused />);
    await waitFor(() => expect(attach).toHaveBeenCalledWith("s-1", "pane-a", expect.any(Function)));
    unmount();
    expect(detach).toHaveBeenCalledWith("s-1");
    expect(fakeTerm.dispose).toHaveBeenCalled();
  });

  it("acks every byte it writes to xterm", async () => {
    render(<TerminalPane sessionId="s-1" paneId="pane-a" focused />);
    await waitFor(() => expect(attach).toHaveBeenCalled());
    const onData = attachMock.mock.calls[0][2];
    onData(new Uint8Array(300));
    expect(ack).toHaveBeenCalledWith("s-1", 300);
  });

  it("ref.focus() focuses xterm and focused defaults to false", async () => {
    const ref = createRef<TerminalPaneHandle>();
    render(<TerminalPane sessionId="s-1" paneId="pane-a" ref={ref} />);
    await waitFor(() => expect(attach).toHaveBeenCalled());
    expect(fakeTerm.focus).not.toHaveBeenCalled();
    ref.current?.focus();
    expect(fakeTerm.focus).toHaveBeenCalledTimes(1);
  });

  it("renders the placeholder when the session is attached elsewhere", () => {
    terminalStore.set({
      "s-1": { status: "elsewhere", attachedTo: { windowId: "w-2", paneId: "pane-b" }, pending: 0 },
    });
    render(<TerminalPane sessionId="s-1" paneId="pane-a" focused />);
    expect(screen.getByText("Bring here")).toBeInTheDocument();
  });

  it("explains that the session is shared, still running and moved rather than restarted", () => {
    terminalStore.set({
      "s-1": { status: "elsewhere", attachedTo: { windowId: "w-1", paneId: "pane-b" }, pending: 0 }, // outside Tauri this window is w-1
    });
    render(<TerminalPane sessionId="s-1" paneId="pane-a" focused />);
    expect(screen.getByText("Open in another pane")).toBeInTheDocument();
    const text = screen.getByText(/already open in another pane, maybe in another view/);
    expect(text).toHaveTextContent("nothing starts a second one");
    expect(text).toHaveTextContent("“Bring here” moves it here without restarting it");
  });

  it("shows the exit line and pending count from the store", () => {
    terminalStore.set({ "s-1": { status: "exited", exitCode: 130, pending: 0 } });
    render(<TerminalPane sessionId="s-1" paneId="pane-a" focused />);
    expect(screen.getByText("Session ended (exit 130)")).toBeInTheDocument();
    act(() => terminalStore.set({ "s-1": { status: "attached", pending: 2 } }));
    expect(screen.getByText("2 prompts will send when idle")).toBeInTheDocument();
  });

  it("sets data-term-theme from the resolved palette", () => {
    settings.current = { terminal_theme: { type: "AlwaysDark" } };
    const { container } = render(<TerminalPane sessionId="s-1" paneId="pane-a" focused />);
    expect(container.firstElementChild?.getAttribute("data-term-theme")).toBe("graphite");
  });
});
