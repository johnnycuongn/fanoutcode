import { useCallback, useState, type ReactElement, type ReactNode } from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui";
import type { ConfirmOptions } from "../types";

export type ConfirmDialogProps = ConfirmOptions & { open: boolean; onResult(ok: boolean): void };

export function ConfirmDialog({ open, title, body, confirmLabel, danger, onResult }: ConfirmDialogProps): ReactElement {
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onResult(false)}>
      <DialogContent size="sm" hideClose>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {body ? (
            <DialogDescription>{body}</DialogDescription>
          ) : (
            <DialogDescription className="sr-only">{title}</DialogDescription>
          )}
        </DialogHeader>
        <DialogFooter>
          <Button onClick={() => onResult(false)}>Cancel</Button>
          <Button variant={danger ? "danger" : "primary"} onClick={() => onResult(true)}>
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** `confirm(opts)` resolves true or false when the developer answers; render `element` once in the board. */
export function useConfirm(): { confirm(opts: ConfirmOptions): Promise<boolean>; element: ReactNode } {
  const [pending, setPending] = useState<{ opts: ConfirmOptions; resolve(ok: boolean): void } | null>(null);
  const confirm = useCallback(
    (opts: ConfirmOptions) =>
      new Promise<boolean>((resolve) =>
        setPending((previous) => {
          previous?.resolve(false);
          return { opts, resolve };
        }),
      ),
    [],
  );
  const settle = (ok: boolean) => {
    pending?.resolve(ok);
    setPending(null);
  };
  const element = pending ? <ConfirmDialog open {...pending.opts} onResult={settle} /> : null;
  return { confirm, element };
}
