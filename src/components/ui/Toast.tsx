import { AlertTriangle, CheckCircle2, Info, XCircle } from "lucide-react";
import type { ReactElement } from "react";
import { Toaster as Sonner } from "sonner";

/** The only toast API: `toast("…")`, `toast.success/error/info/warning/promise`, `{ action, description, duration }`. */
export { toast } from "sonner";

export type ToasterProps = {
  position?: "bottom-right" | "bottom-center" | "top-right";
  theme?: "light" | "dark" | "system";
};

/** Sonner, unstyled, with the semantic tokens. Mounted once by UiProvider. */
export function Toaster({ position = "bottom-right", theme = "system" }: ToasterProps): ReactElement {
  return (
    <Sonner
      position={position}
      theme={theme}
      closeButton
      duration={4000}
      visibleToasts={4}
      icons={{
        success: <CheckCircle2 aria-hidden className="size-4 text-ok" />,
        error: <XCircle aria-hidden className="size-4 text-bad" />,
        warning: <AlertTriangle aria-hidden className="size-4 text-warn" />,
        info: <Info aria-hidden className="size-4 text-accent" />,
      }}
      toastOptions={{
        unstyled: true,
        classNames: {
          toast:
            "flex w-[356px] items-center gap-2 rounded-well border border-line bg-card p-3 text-sm text-ink shadow-card",
          title: "font-medium text-ink",
          description: "text-xs text-mute",
          actionButton: "ml-auto rounded-card bg-accent px-2 py-1 text-xs font-medium text-accent-ink",
          cancelButton: "ml-auto rounded-card bg-well px-2 py-1 text-xs text-ink",
          closeButton:
            "absolute -top-1.5 -left-1.5 flex size-4 items-center justify-center rounded-full border border-line bg-card text-mute",
        },
      }}
    />
  );
}
