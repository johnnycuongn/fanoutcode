// Shared by Menu and ContextMenu (not exported from the barrel): item classes and the icon / hint / shortcut layout.
import type { ReactElement, ReactNode } from "react";
import { cx } from "./cx";
import { KeyHint } from "./KeyHint";
import { swatchClasses, type Swatch } from "./tokens";

export const menuContentClasses = "z-50 rounded-well border border-line bg-card p-1 text-ink shadow-card outline-none";

export const menuItemClasses =
  "relative flex h-7 cursor-default select-none items-center gap-2 rounded-card px-2 text-sm text-ink outline-none data-[highlighted]:bg-well data-[disabled]:pointer-events-none data-[disabled]:opacity-50";

export function menuItemTone(danger?: boolean): string {
  return danger ? "text-bad data-[highlighted]:bg-bad/10" : "";
}

export function MenuItemBody({
  icon,
  hint,
  shortcut,
  children,
}: {
  icon?: ReactNode;
  hint?: ReactNode;
  shortcut?: string;
  children?: ReactNode;
}): ReactElement {
  return (
    <>
      {icon && <span className="flex size-3.5 shrink-0 items-center justify-center [&>svg]:size-3.5">{icon}</span>}
      <span className="min-w-0 flex-1 truncate">{children}</span>
      {(hint || shortcut) && (
        <span className="ml-auto flex items-center gap-2 pl-4 text-mute">
          {hint && <span className="text-xs">{hint}</span>}
          {shortcut && <KeyHint keys={shortcut} size="sm" />}
        </span>
      )}
    </>
  );
}

export function SwatchSquare({ swatch }: { swatch: Swatch }): ReactElement {
  return <span aria-hidden className={cx("size-2.5 shrink-0 rounded-sm", swatchClasses[swatch].solid)} />;
}
