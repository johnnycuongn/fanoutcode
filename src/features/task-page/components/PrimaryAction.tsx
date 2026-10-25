// The header's primary button (⌘⏎), the queued hint while the agent works, and a split menu to start fresh instead.
import { ChevronDown } from "lucide-react";
import type { ReactElement } from "react";
import { Button, Chip, IconButton, KeyHint, Menu, MenuContent, MenuItem, MenuTrigger } from "@/components/ui";
import type { Stage, StageAction } from "@/lib/bindings";
import { freshSessionAllowed, type PrimaryButton } from "../primaryAction";

export interface PrimaryActionProps {
  primary: PrimaryButton;
  stage: Stage;
  onPrimary(): void;
  onAction(a: StageAction): void;
}

export function PrimaryAction({ primary, stage, onPrimary, onAction }: PrimaryActionProps): ReactElement {
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
          className={freshSessionAllowed(stage) ? "rounded-r-none" : undefined}
          trailing={<KeyHint aria-hidden keys="cmd+enter" size="sm" className="opacity-80" />}
        >
          {primary.label}
        </Button>
        {freshSessionAllowed(stage) && (
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
              <MenuItem onSelect={() => onAction({ type: "StartFresh" })}>Start fresh session</MenuItem>
            </MenuContent>
          </Menu>
        )}
      </div>
    </div>
  );
}
