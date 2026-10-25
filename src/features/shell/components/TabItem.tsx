// One view tab: a beating status dot and a stage icon, its name, the amber needs-you count, the home-project dots
// (workspace windows), double-click or the context menu to rename, and Close view (confirmed when the view shows tasks). Dragging reorders.
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
  StatusDot,
  TabsTrigger,
  cx,
  statusLabel,
  type AgentStatus,
} from "@/components/ui";
import type { View } from "@/lib/bindings";
import { useReducedMotion } from "@/lib/motion";
import { swatchClass } from "@/lib/theme";
import { deleteView, renameView, viewTaskIds } from "@/features/views";
import { setActiveTab } from "../store";
import { StageIcon, type StageIconKind } from "./StageIcon";

export type TabItemProps = {
  view: View;
  active: boolean;
  count: number;
  colors: string[];
  /** The view's aggregate task state and stage; null while it shows no task. */
  status: AgentStatus | null;
  stage: StageIconKind | null;
};

function tooltip(status: AgentStatus | null, stage: StageIconKind | null): string | undefined {
  if (!status || !stage) return undefined;
  return `${stage === "many" ? "Several stages" : stage} · ${statusLabel(status)}`;
}

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

export function TabItem({ view, active, count, colors, status, stage }: TabItemProps): ReactElement {
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
            title={tooltip(status, stage)}
          >
            {status && <StatusDot status={status} pulse={status !== "idle"} size="sm" aria-hidden />}
            {stage && <StageIcon stage={stage} />}
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
