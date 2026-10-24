import type { ReactElement } from "react";
import {
  ContextMenu,
  ContextMenuCheckboxItem,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuLabel,
  ContextMenuRadioGroup,
  ContextMenuRadioItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "../../../components/ui";

export default function ContextMenuPage(): ReactElement {
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold">ContextMenu</h1>
      <ContextMenu>
        <ContextMenuTrigger asChild>
          <div className="grid h-24 w-64 place-items-center rounded-card border border-line bg-card text-sm text-mute">
            Right-click this card
          </div>
        </ContextMenuTrigger>
        <ContextMenuContent>
          <ContextMenuItem>Open in current view</ContextMenuItem>
          <ContextMenuItem shortcut="cmd+e">Rename</ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuLabel>Priority</ContextMenuLabel>
          <ContextMenuRadioGroup value="medium">
            <ContextMenuRadioItem value="low">Low</ContextMenuRadioItem>
            <ContextMenuRadioItem value="medium">Medium</ContextMenuRadioItem>
          </ContextMenuRadioGroup>
          <ContextMenuCheckboxItem checked>Flag</ContextMenuCheckboxItem>
          <ContextMenuSeparator />
          <ContextMenuItem danger shortcut="cmd+shift+a">
            Archive
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>
    </div>
  );
}
