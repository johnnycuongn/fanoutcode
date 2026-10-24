import type { ReactElement, ReactNode } from "react";
import { Toaster } from "./Toast";
import { TooltipProvider } from "./Tooltip";

export type UiProviderProps = { children: ReactNode };

/** Mounted once at each window's root: the tooltip provider and the toaster. */
export function UiProvider({ children }: UiProviderProps): ReactElement {
  return (
    <TooltipProvider>
      {children}
      <Toaster />
    </TooltipProvider>
  );
}
