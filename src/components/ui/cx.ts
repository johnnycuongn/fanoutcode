import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import type { Size } from "./tokens";

/** Joins class names and resolves conflicting Tailwind utilities, last one wins. */
export function cx(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

/** Keyboard focus ring for every focusable element; never remove it. */
export const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-accent/60 focus-visible:ring-offset-1 focus-visible:ring-offset-page";

export const controlHeight: Record<Size, string> = { sm: "h-6", md: "h-7", lg: "h-8" };
export const controlText: Record<Size, string> = { sm: "text-xs", md: "text-sm", lg: "text-sm" };

/** Surface shared by text-entry controls and select triggers. */
export const controlSurface =
  "rounded-card border border-line bg-card px-2 text-ink placeholder:text-mute aria-invalid:border-bad disabled:opacity-50";

/**
 * Open/close animation for floating content (popover, menu, select, dialog). Full motion fades and zooms over
 * 200 ms; reduced motion fades over 150 ms with no zoom (spec §11.3). Literal strings so Tailwind sees them.
 */
export function popMotion(reduced: boolean): string {
  return reduced
    ? "data-[state=open]:animate-in data-[state=closed]:animate-out duration-150 ease-out data-[state=open]:fade-in-0 data-[state=closed]:fade-out-0"
    : "data-[state=open]:animate-in data-[state=closed]:animate-out duration-200 data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95";
}
