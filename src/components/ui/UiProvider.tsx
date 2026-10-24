import type { ReactElement, ReactNode } from "react";
import { TooltipProvider } from "./Tooltip";

export type UiProviderProps = { children: ReactNode };

/** Mounted once at each window's root: the tooltip provider (and the toaster, from Task 13). */
export function UiProvider({ children }: UiProviderProps): ReactElement {
  return <TooltipProvider>{children}</TooltipProvider>;
}
