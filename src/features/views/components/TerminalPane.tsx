// A session's terminal (A5): one xterm attached through terminalStore, coloured from the `--term-*` variables of the
// palette the terminal-theme setting picks, with a status line for the exit and the prompts waiting for idle. A
// session held by another pane shows TerminalPlaceholder over the (idle) terminal instead.
import { useEffect, useImperativeHandle, useLayoutEffect, useRef, useState, type ReactElement, type Ref } from "react";
import type { ITheme } from "@xterm/xterm";
import type { TerminalTheme } from "@/lib/bindings";
import { useAppTheme } from "@/lib/theme";
import { useSettings } from "@/features/settings";
import { bring, startListening, useTerminal } from "../terminalStore";
import { readXtermTheme, rendererFor, resolvePalette } from "../terminalTheme";
import { useXterm } from "../useXterm";
import { TerminalPlaceholder } from "./TerminalPlaceholder";

export type TerminalPaneProps = { sessionId: string; paneId: string; focused?: boolean };
export type TerminalPaneHandle = { focus(): void };

const FOLLOW_APP: TerminalTheme = { type: "FollowApp" };
const DEFAULT_FONT = "JetBrains Mono";
/** The bundled face behind the setting's family name (Fontsource ships it as a variable font). */
const FONT_FALLBACK = '"JetBrains Mono Variable", ui-monospace, "SF Mono", Menlo, monospace';

function statusLine(status: string, exitCode: number | undefined, pending: number): string | null {
  if (status === "exited") return `Session ended (exit ${exitCode ?? "?"})`;
  if (pending === 1) return "1 prompt will send when idle";
  if (pending > 1) return `${pending} prompts will send when idle`;
  return null;
}

export function TerminalPane({
  sessionId,
  paneId,
  focused = false,
  ref,
}: TerminalPaneProps & { ref?: Ref<TerminalPaneHandle> }): ReactElement {
  const settings = useSettings();
  const appTheme = useAppTheme();
  const state = useTerminal(sessionId);
  const palette = resolvePalette(settings?.terminal_theme ?? FOLLOW_APP, appTheme);
  const rootRef = useRef<HTMLDivElement | null>(null);
  const [theme, setTheme] = useState<ITheme>({});
  const family = settings?.terminal_font_family || DEFAULT_FONT;

  // the palette's variables apply once data-term-theme is in the DOM, so read them after layout
  useLayoutEffect(() => {
    if (rootRef.current) setTheme(readXtermTheme(rootRef.current));
  }, [palette]);
  useEffect(() => {
    startListening();
  }, []);

  const { containerRef, term, deliver } = useXterm({
    sessionId,
    paneId,
    renderer: rendererFor(settings?.terminal_ligatures ?? false),
    theme,
    fontFamily: `"${family}", ${FONT_FALLBACK}`,
    fontSize: settings?.terminal_font_size ?? 13,
    scrollback: settings?.scrollback_lines ?? 10_000,
  });

  useImperativeHandle(ref, () => ({ focus: () => term?.focus() }), [term]);
  useEffect(() => {
    if (focused) term?.focus();
  }, [focused, term]);

  const onBring = () => {
    term?.reset();
    bring(sessionId, paneId, deliver).catch((err: unknown) => console.error("Fanout Code: bring failed", err));
  };
  const elsewhere = state.status === "elsewhere" && state.attachedTo;
  const line = statusLine(state.status, state.exitCode, state.pending);

  return (
    <div ref={rootRef} data-term-theme={palette} className="relative flex h-full w-full flex-col">
      <div ref={containerRef} className="min-h-0 flex-1 overflow-hidden px-2 pt-1" aria-hidden={!!elsewhere} />
      {line && <div className="shrink-0 px-2 py-1 font-mono text-xs text-mute">{line}</div>}
      {elsewhere && (
        <div className="absolute inset-0 bg-page">
          <TerminalPlaceholder
            sessionId={sessionId}
            attachedTo={elsewhere}
            onBring={onBring}
            bringLabel={state.broughtAway ? "Bring back" : "Bring here"}
          />
        </div>
      )}
    </div>
  );
}
