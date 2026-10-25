// Where the workspace file lives and its save state: write-through in app data, or explicit save elsewhere with
// the Unsaved chip, Save / Save as…, Re-export… when the file is gone, and the reload banner for edits made on disk.
import { useState, type ReactElement } from "react";
import {
  Badge,
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui";
import { pickSavePath } from "../pickers";
import { workspaceStore } from "../store";
import { Section } from "./Section";

function ReloadBanner(): ReactElement {
  const [confirming, setConfirming] = useState(false);
  const reload = () => {
    if (workspaceStore.isDirty()) setConfirming(true);
    else void workspaceStore.reload(false);
  };
  return (
    <div role="status" className="flex items-center justify-between gap-2 rounded-well bg-well p-2 text-sm">
      <span className="text-warn">This workspace file changed on disk.</span>
      <span className="flex gap-1">
        <Button size="sm" onClick={reload}>
          Reload
        </Button>
        <Button size="sm" variant="ghost" onClick={() => workspaceStore.keepMine()}>
          Keep mine
        </Button>
      </span>
      <Dialog open={confirming} onOpenChange={setConfirming}>
        <DialogContent size="sm">
          <DialogHeader>
            <DialogTitle>Discard unsaved changes?</DialogTitle>
            <DialogDescription>
              Reloading replaces your unsaved workspace changes with the file on disk.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setConfirming(false)}>
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={() => {
                setConfirming(false);
                void workspaceStore.reload(true);
              }}
            >
              Discard and reload
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export function WorkspaceFileSection(): ReactElement | null {
  const { workspace: ws, status, externalChange, busy, error } = workspaceStore.use();
  if (!ws) return null;
  const explicit = status?.explicit_save ?? false;
  const saveAs = async () => {
    const path = await pickSavePath(`${ws.name}.fanoutcode-workspace.json`);
    if (path) await workspaceStore.saveAs(path);
  };
  return (
    <Section title="File">
      <div className="flex items-center gap-2">
        <span className="truncate font-mono text-xs text-mute" title={ws.file_path ?? undefined}>
          {ws.file_path ?? "Not saved yet"}
        </span>
        {ws.dirty && (
          <Badge tone="warn" size="sm">
            Unsaved
          </Badge>
        )}
      </div>
      <p className="text-xs text-mute">
        {explicit ? "Saved on ⌘S and when the window closes" : "Written automatically (app data)"}
      </p>
      <div className="flex gap-1">
        <Button size="sm" disabled={busy || !ws.dirty} onClick={() => void workspaceStore.save()}>
          Save
        </Button>
        <Button size="sm" variant="secondary" disabled={busy} onClick={() => void saveAs()}>
          Save as…
        </Button>
        {status?.file_missing && (
          <Button size="sm" variant="secondary" disabled={busy} onClick={() => void saveAs()}>
            Re-export…
          </Button>
        )}
      </div>
      {externalChange && <ReloadBanner />}
      {error && <p className="text-xs text-bad">{error}</p>}
    </Section>
  );
}
