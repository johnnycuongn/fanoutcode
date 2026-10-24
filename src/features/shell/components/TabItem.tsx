// One view tab: its name, the amber needs-you count, the home-project dots (workspace windows), double-click or the
// context menu to rename, and Close view (confirmed when the view shows tasks). Dragging reorders.
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { useState, type ReactElement } from "react";
import {
  Badge,
  Button,
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  TabsTrigger,
  cx,
} from "@/components/ui";
import type { View } from "@/lib/bindings";
import { useReducedMotion } from "@/lib/motion";
import { swatchClass } from "@/lib/theme";
import { deleteView, renameView, viewTaskIds } from "@/features/views";
import { setActiveTab } from "../store";

export type TabItemProps = { view: View; active: boolean; count: number; colors: string[] };

function RenameInput({ view, onDone }: { view: View; onDone(): void }): ReactElement {
  const [name, setName] = useState(view.name);
  const commit = () => {
    if (name.trim() && name.trim() !== view.name) void renameView(view.id, name);
    onDone();
  };
  return (
    <Input
      autoFocus
      size="sm"
      aria-label="View name"
      value={name}
      className="mx-0.5 mt-1 h-6 w-28"
      onChange={(e) => setName(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") commit();
        if (e.key === "Escape") onDone();
      }}
    />
  );
}

export function TabItem({ view, active, count, colors }: TabItemProps): ReactElement {
  const [renaming, setRenaming] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const reduced = useReducedMotion();
  const { setNodeRef, attributes, listeners, transform, transition, isDragging } = useSortable({ id: view.id });
  const close = () => (viewTaskIds(view).length > 0 ? setConfirming(true) : void deleteView(view.id));
  if (renaming) return <RenameInput view={view} onDone={() => setRenaming(false)} />;
  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger asChild>
          <TabsTrigger
            ref={setNodeRef}
            value={view.id}
            data-tab-id={view.id}
            style={{ transform: CSS.Translate.toString(transform), transition: reduced ? undefined : transition }}
            className={cx(isDragging && "opacity-60")}
            {...attributes}
            {...listeners}
            role="tab"
            aria-selected={active}
            onClick={() => setActiveTab(view.id)}
            onDoubleClick={() => setRenaming(true)}
          >
            {colors.map((c, i) => (
              <span key={`${i}-${c}`} aria-hidden className={cx("size-1.5 rounded-full", swatchClass(c, "bg"))} />
            ))}
            {view.name}
            {count > 0 && (
              <Badge tone="warn" size="sm" data-tone="warn">
                {count}
              </Badge>
            )}
          </TabsTrigger>
        </ContextMenuTrigger>
        <ContextMenuContent>
          <ContextMenuItem onSelect={() => setRenaming(true)}>Rename</ContextMenuItem>
          <ContextMenuItem danger onSelect={close}>
            Close view
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>
      <Dialog open={confirming} onOpenChange={setConfirming}>
        <DialogContent size="sm">
          <DialogHeader>
            <DialogTitle>Close “{view.name}”?</DialogTitle>
            <DialogDescription>Its tasks keep running; only this arrangement of panes goes away.</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="secondary" onClick={() => setConfirming(false)}>
              Cancel
            </Button>
            <Button variant="danger" onClick={() => void deleteView(view.id).then(() => setConfirming(false))}>
              Close view
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
