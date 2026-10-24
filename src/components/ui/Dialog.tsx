import { X } from "lucide-react";
import { Dialog as DialogPrimitive } from "radix-ui";
import type { ComponentProps, ReactElement } from "react";
import { useReducedMotion } from "@/lib/motion";
import { cx, popMotion } from "./cx";
import { IconButton } from "./IconButton";

export const Dialog = DialogPrimitive.Root;
export const DialogTrigger = DialogPrimitive.Trigger;
export const DialogClose = DialogPrimitive.Close;
export const DialogPortal = DialogPrimitive.Portal;

export type DialogContentProps = ComponentProps<typeof DialogPrimitive.Content> & {
  size?: "sm" | "md" | "lg";
  /** Hides the top-right Close button. */
  hideClose?: boolean;
};

const SIZE = { sm: "max-w-[360px]", md: "max-w-[480px]", lg: "max-w-[640px]" } as const;

const FADE_REDUCED =
  "data-[state=open]:animate-in data-[state=closed]:animate-out duration-150 ease-out data-[state=open]:fade-in-0 data-[state=closed]:fade-out-0";

/** Portal > Overlay + Content. Radix gives the focus trap, Escape, focus return and stacking of nested dialogs. */
export function DialogContent({
  size = "md",
  hideClose = false,
  className,
  children,
  ...props
}: DialogContentProps): ReactElement {
  const reduced = useReducedMotion();
  const motion = reduced ? "reduced" : "full";
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Overlay
        data-motion={motion}
        className={cx(
          "fixed inset-0 z-50 bg-ink/40",
          reduced
            ? FADE_REDUCED
            : "data-[state=open]:animate-in data-[state=closed]:animate-out duration-200 data-[state=open]:fade-in-0 data-[state=closed]:fade-out-0",
        )}
      />
      <DialogPrimitive.Content
        data-motion={motion}
        className={cx(
          "fixed top-1/2 left-1/2 z-50 w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 rounded-well border border-line bg-card p-4 text-ink shadow-card outline-none",
          SIZE[size],
          popMotion(reduced),
          className,
        )}
        {...props}
      >
        {children}
        {!hideClose && (
          <DialogPrimitive.Close asChild>
            <IconButton
              label="Close"
              size="sm"
              icon={<X aria-hidden className="size-3.5" />}
              className="absolute top-3 right-3"
            />
          </DialogPrimitive.Close>
        )}
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  );
}

export function DialogHeader({ className, ...props }: ComponentProps<"div">): ReactElement {
  return <div className={cx("mb-3 flex flex-col gap-1 pr-8", className)} {...props} />;
}

export function DialogTitle({ className, ...props }: ComponentProps<typeof DialogPrimitive.Title>): ReactElement {
  return <DialogPrimitive.Title className={cx("font-medium text-ink", className)} {...props} />;
}

export function DialogDescription({
  className,
  ...props
}: ComponentProps<typeof DialogPrimitive.Description>): ReactElement {
  return <DialogPrimitive.Description className={cx("text-sm text-mute", className)} {...props} />;
}

export function DialogFooter({ className, ...props }: ComponentProps<"div">): ReactElement {
  return <div className={cx("mt-4 flex justify-end gap-2", className)} {...props} />;
}
