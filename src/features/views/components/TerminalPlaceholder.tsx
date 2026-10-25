// What a terminal pane shows while its session is attached in another pane (§7.2: one attachment per session).
import { useEffect, useState, type ReactElement } from "react";
import { Button, EmptyState } from "@/components/ui";
import { commands } from "@/lib/bindings";
import { currentWindowId } from "@/lib/windowId";

export type TerminalPlaceholderProps = {
  sessionId: string;
  attachedTo: { windowId: string; paneId: string };
  onBring: () => void;
  /** "Bring back" when this pane held the session before it was brought elsewhere. */
  bringLabel?: string;
};

export function TerminalPlaceholder({
  sessionId,
  attachedTo,
  onBring,
  bringLabel = "Bring here",
}: TerminalPlaceholderProps): ReactElement {
  const [title, setTitle] = useState<string | null>(null);
  const otherWindow = attachedTo.windowId !== currentWindowId();
  useEffect(() => {
    if (!otherWindow) return;
    let live = true;
    void Promise.resolve(commands.windowsList()).then((windows) => {
      const list = Array.isArray(windows) ? windows : [];
      const w = list.find((x) => x.window_id === attachedTo.windowId);
      if (live) setTitle(w?.title ?? null);
    });
    return () => {
      live = false;
    };
  }, [otherWindow, attachedTo.windowId]);
  return (
    <EmptyState
      className="h-full justify-center"
      data-session={sessionId}
      title={otherWindow ? `Open in ${title ?? "another window"}` : "Open in another pane"}
      description="A terminal shows in one pane at a time."
      action={
        <div className="flex gap-2">
          {otherWindow && (
            <Button variant="secondary" size="sm" onClick={() => void commands.windowsFocus(attachedTo.windowId)}>
              Show window
            </Button>
          )}
          <Button size="sm" onClick={onBring}>
            {bringLabel}
          </Button>
        </div>
      }
    />
  );
}
