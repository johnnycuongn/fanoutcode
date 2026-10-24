// STUB(plan 04): the smallest TerminalPane with Plan 04's exact props and handle (A5); Plan 04 replaces this file.
import { useImperativeHandle, type ReactElement, type Ref } from "react";
import { Panel } from "@/components/ui";

export type TerminalPaneProps = { sessionId: string; paneId: string; focused?: boolean };
export type TerminalPaneHandle = { focus(): void };

export function TerminalPane({ sessionId, ref }: TerminalPaneProps & { ref?: Ref<TerminalPaneHandle> }): ReactElement {
  useImperativeHandle(ref, () => ({ focus: () => {} }));
  return (
    <Panel variant="plain" className="h-full font-mono text-mute">
      Terminal {sessionId}
    </Panel>
  );
}
