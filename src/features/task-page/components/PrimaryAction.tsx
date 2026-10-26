// The header's primary button (⌘⏎), the queued hint while the agent works, and a split menu with the stage's forward
// step (when the button does something else, such as Reply) and Start fresh session.
import { ChevronDown } from "lucide-react";
import type { ReactElement } from "react";
import { Button, Chip, IconButton, KeyHint, Menu, MenuContent, MenuItem, MenuTrigger } from "@/components/ui";
import type { Stage, StageAction } from "@/lib/bindings";
import { forwardActionFor, freshSessionAllowed, type PrimaryButton } from "../primaryAction";

export interface PrimaryActionProps {
  primary: PrimaryButton;
  stage: Stage;
  mayCommit: boolean;
  onPrimary(): void;
  onAction(a: StageAction): void;
}

export function PrimaryAction({ primary, stage, mayCommit, onPrimary, onAction }: PrimaryActionProps): ReactElement {
  const forward = forwardActionFor(stage, { mayCommit });
  // the forward step is always one click away, even while the button answers the agent or a decision
  const next = forward && forward.label !== primary.label ? forward : null;
  const fresh = freshSessionAllowed(stage);
  const split = fresh || next !== null;
  return (
    <div className="flex items-center gap-1.5">
      {primary.hint && (
        <Chip size="sm" tone="neutral">
          {primary.hint}
        </Chip>
      )}
      <div className="flex items-center">
        <Button
          variant="primary"
          size="sm"
          disabled={primary.disabled}
          onClick={onPrimary}
          className={split ? "rounded-r-none" : undefined}
          trailing={<KeyHint aria-hidden keys="cmd+enter" size="sm" className="opacity-80" />}
        >
          {primary.label}
        </Button>
        {split && (
          <Menu>
            <MenuTrigger asChild>
              <IconButton
                size="sm"
                variant="primary"
                label="Other ways to continue"
                className="rounded-l-none border-l border-page/30"
                icon={<ChevronDown aria-hidden className="size-3.5" />}
              />
            </MenuTrigger>
            <MenuContent align="end">
              {next && <MenuItem onSelect={() => onAction(next.action)}>{next.label}</MenuItem>}
              {fresh && <MenuItem onSelect={() => onAction({ type: "StartFresh" })}>Start fresh session</MenuItem>}
            </MenuContent>
          </Menu>
        )}
      </div>
    </div>
  );
}
