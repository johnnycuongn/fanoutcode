// The Spec stage's terminal: a 28 px strip under the editor, or (expanded) a bar above the terminal itself. Without a
// session it offers to start one.
import { ChevronDown, ChevronUp } from "lucide-react";
import type { ReactElement } from "react";
import { Button, IconButton, KeyHint } from "@/components/ui";

export interface TerminalStripProps {
  expanded: boolean;
  hasSession: boolean;
  onToggle(): void;
  onStart(): void;
}

export function TerminalStrip({ expanded, hasSession, onToggle, onStart }: TerminalStripProps): ReactElement {
  return (
    <div className="flex h-7 shrink-0 items-center gap-2 border-t border-line bg-bar px-2 text-xs text-mute">
      <span className="font-medium text-ink">Terminal</span>
      <KeyHint keys="ctrl+`" size="sm" />
      <span className="flex-1" />
      {!hasSession && (
        <Button variant="secondary" size="sm" onClick={onStart}>
          Start session
        </Button>
      )}
      <IconButton
        size="sm"
        label={expanded ? "Collapse terminal" : "Expand terminal"}
        icon={
          expanded ? <ChevronDown aria-hidden className="size-3.5" /> : <ChevronUp aria-hidden className="size-3.5" />
        }
        onClick={onToggle}
      />
    </div>
  );
}
