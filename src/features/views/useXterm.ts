// One xterm instance per mounted terminal pane: construction, addons (fit, WebGL with a canvas fallback when the
// GPU context is lost), the resize observer, keystrokes out, bytes in with flow-control acks, and attach/detach.
// Every xterm detail lives here so TerminalPane stays a layout component. xterm and its addons load as their own
// chunk on the first terminal, so the shell's first paint does not carry them.
import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import type { ITheme, Terminal } from "@xterm/xterm";
import type { FitAddon } from "@xterm/addon-fit";
import { isAppChord } from "@/lib/keyboard";
import { ack, attach, detach, resize, write } from "./terminalStore";

export type XtermOptions = {
  sessionId: string;
  paneId: string;
  renderer: "webgl" | "dom";
  theme: ITheme;
  fontFamily: string;
  fontSize: number;
  scrollback: number;
};

type Modules = {
  Terminal: typeof import("@xterm/xterm").Terminal;
  FitAddon: typeof import("@xterm/addon-fit").FitAddon;
  WebglAddon: typeof import("@xterm/addon-webgl").WebglAddon;
  CanvasAddon: typeof import("@xterm/addon-canvas").CanvasAddon;
};

let modules: Promise<Modules> | null = null;
function loadModules(): Promise<Modules> {
  modules ??= Promise.all([
    import("@xterm/xterm"),
    import("@xterm/addon-fit"),
    import("@xterm/addon-webgl"),
    import("@xterm/addon-canvas"),
  ]).then(([x, f, w, c]) => ({
    Terminal: x.Terminal,
    FitAddon: f.FitAddon,
    WebglAddon: w.WebglAddon,
    CanvasAddon: c.CanvasAddon,
  }));
  return modules;
}

/** xterm's binary strings carry one byte per char code. */
function binaryBytes(data: string): Uint8Array {
  const out = new Uint8Array(data.length);
  for (let i = 0; i < data.length; i++) out[i] = data.charCodeAt(i) & 0xff;
  return out;
}

type Gpu = { dispose(): void };

/** Loads the GPU renderer; a lost WebGL context falls back to the canvas renderer. */
function loadGpu(m: Modules, term: Terminal, slot: { current: Gpu | null }): void {
  try {
    const webgl = new m.WebglAddon();
    webgl.onContextLoss(() => {
      webgl.dispose();
      const canvas = new m.CanvasAddon();
      term.loadAddon(canvas);
      slot.current = canvas;
    });
    term.loadAddon(webgl);
    slot.current = webgl;
  } catch {
    slot.current = null; // no WebGL here: xterm's DOM renderer stays
  }
}

/** Fits the terminal to its box; a collapsed pane (0 columns or rows) never reaches the PTY. */
function fitAndResize(sessionId: string, term: Terminal, fit: FitAddon): void {
  try {
    fit.fit();
  } catch {
    return; // not laid out yet
  }
  if (term.cols > 0 && term.rows > 0) resize(sessionId, term.cols, term.rows);
}

export function useXterm(opts: XtermOptions): {
  containerRef: RefObject<HTMLDivElement | null>;
  term: Terminal | null;
  /** Feeds xterm and acks what it drew; `bring` passes it to the store again. */
  deliver: (bytes: Uint8Array) => void;
} {
  const { sessionId, paneId, renderer, theme, fontFamily, fontSize, scrollback } = opts;
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [term, setTerm] = useState<Terminal | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const gpuRef = useRef<Gpu | null>(null);
  const modsRef = useRef<Modules | null>(null);
  const latest = useRef({ theme, fontFamily, fontSize, scrollback, renderer });
  useEffect(() => {
    latest.current = { theme, fontFamily, fontSize, scrollback, renderer };
  });

  const deliver = useCallback(
    (bytes: Uint8Array) => termRef.current?.write(bytes, () => ack(sessionId, bytes.byteLength)),
    [sessionId],
  );

  useEffect(() => {
    let disposed = false;
    let cleanup = () => {};
    void loadModules().then((m) => {
      const el = containerRef.current;
      if (disposed || !el) return;
      const o = latest.current;
      const t = new m.Terminal({
        allowProposedApi: true,
        scrollback: o.scrollback,
        fontFamily: o.fontFamily,
        fontSize: o.fontSize,
        theme: o.theme,
        cursorBlink: true,
      });
      const fit = new m.FitAddon();
      t.loadAddon(fit);
      t.open(el);
      if (o.renderer === "webgl") loadGpu(m, t, gpuRef);
      t.attachCustomKeyEventHandler((e) => !isAppChord(e));
      const subs = [
        t.onData((data) => write(sessionId, data)),
        t.onBinary((data) => write(sessionId, binaryBytes(data))),
      ];
      modsRef.current = m;
      termRef.current = t;
      fitRef.current = fit;
      setTerm(t);
      let frame = 0; // fit at most once per animation frame
      const refit = () => {
        frame ||= requestAnimationFrame(() => {
          frame = 0;
          fitAndResize(sessionId, t, fit);
        });
      };
      const observer = new ResizeObserver(refit);
      observer.observe(el);
      refit();
      attach(sessionId, paneId, deliver).catch((err: unknown) => console.error("Team Code: attach failed", err));
      cleanup = () => {
        if (frame) cancelAnimationFrame(frame);
        observer.disconnect();
        subs.forEach((s) => s.dispose());
        void detach(sessionId);
        termRef.current = null;
        fitRef.current = null;
        gpuRef.current = null;
        t.dispose();
      };
    });
    return () => {
      disposed = true;
      cleanup();
      setTerm(null);
    };
  }, [sessionId, paneId, deliver]);

  // `term` (state) re-runs these once the instance exists; the instance itself is reached through the ref
  useEffect(() => {
    const t = termRef.current;
    if (t) t.options.theme = theme;
  }, [term, theme]);

  useEffect(() => {
    const t = termRef.current;
    if (!t) return;
    t.options.fontFamily = fontFamily;
    t.options.fontSize = fontSize;
    t.options.scrollback = scrollback;
    if (fitRef.current) fitAndResize(sessionId, t, fitRef.current);
  }, [term, sessionId, fontFamily, fontSize, scrollback]);

  useEffect(() => {
    const t = termRef.current;
    const m = modsRef.current;
    if (!t || !m) return;
    if (renderer === "webgl" && !gpuRef.current) loadGpu(m, t, gpuRef);
    if (renderer === "dom" && gpuRef.current) {
      gpuRef.current.dispose();
      gpuRef.current = null;
    }
  }, [term, renderer]);

  return { containerRef, term, deliver };
}
