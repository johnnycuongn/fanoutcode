// The review of a workspace file before it is imported: each project folder, the linking policy in plain words, and
// what the file adds (agents, prompt instructions, settings, references). Every reference is approved one by one.
// Mounted once at the window root; `openWorkspaceFile` fills it.
import type { ReactElement } from "react";
import {
  Badge,
  Button,
  Checkbox,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui";
import type { WorkspaceImportPreview } from "@/lib/bindings";
import { importStore } from "../importStore";

function policyWords(p: WorkspaceImportPreview): string {
  return p.policy === "ReadOnly"
    ? "Agents can read these folders but not edit them (read only)"
    : "Agents can read and edit these folders";
}

function Additions({ p }: { p: WorkspaceImportPreview }): ReactElement | null {
  const lines = [
    p.agent_names.length > 0 && `Adds custom agents: ${p.agent_names.join(", ")}`,
    p.fragment_stages.length > 0 && `Adds prompt instructions for: ${p.fragment_stages.join(", ")}`,
    p.settings_keys.length > 0 && `Sets: ${p.settings_keys.join(", ")}`,
  ].filter(Boolean) as string[];
  if (lines.length === 0) return null;
  return (
    <ul className="flex flex-col gap-0.5 text-sm text-warn">
      {lines.map((l) => (
        <li key={l}>{l}</li>
      ))}
    </ul>
  );
}

function References({ p, approved }: { p: WorkspaceImportPreview; approved: string[] }): ReactElement | null {
  if (p.docs.length === 0) return null;
  return (
    <ul className="flex flex-col gap-1 text-sm" aria-label="References">
      {p.docs.map((d) => (
        <li key={d.id} className="flex flex-col gap-0.5">
          <Checkbox
            aria-label={`Allow reading ${d.target}`}
            label={
              <span className="font-mono text-xs">{d.host ? `${d.label} — ${d.host} (${d.target})` : d.target}</span>
            }
            checked={approved.includes(d.id)}
            onCheckedChange={() => importStore.toggle(d.id)}
          />
          {d.outside_projects && <span className="pl-6 text-xs text-warn">Outside the workspace's projects</span>}
        </li>
      ))}
    </ul>
  );
}

export function WorkspaceImportDialog(): ReactElement | null {
  const { preview, approved, busy, error } = importStore.use();
  if (!preview) return null;
  return (
    <Dialog open onOpenChange={(open) => !open && importStore.cancel()}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{`Open “${preview.name}”?`}</DialogTitle>
          <DialogDescription>{policyWords(preview)}</DialogDescription>
        </DialogHeader>
        <div className="flex max-h-[60vh] flex-col gap-3 overflow-auto">
          <ul className="flex flex-col gap-1" aria-label="Project folders">
            {preview.roots.map((r) => (
              <li key={r.path} className="flex flex-col">
                <span className="flex items-center gap-2">
                  <span className="truncate font-mono text-xs text-ink">{r.path}</span>
                  {r.new_to_app && <Badge size="sm">New</Badge>}
                  {r.protected && <Badge size="sm">Protected</Badge>}
                </span>
                {r.warning && <span className="text-xs text-warn">{r.warning}</span>}
              </li>
            ))}
          </ul>
          <Additions p={preview} />
          <References p={preview} approved={approved} />
          {preview.problems.length > 0 && (
            <ul className={preview.blocked ? "text-sm text-bad" : "text-xs text-mute"} aria-label="Problems">
              {preview.problems.map((m) => (
                <li key={m}>{m}</li>
              ))}
            </ul>
          )}
          {error && <p className="text-sm text-bad">{error}</p>}
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => importStore.cancel()}>
            Cancel
          </Button>
          <Button disabled={preview.blocked} loading={busy} onClick={() => void importStore.confirm()}>
            Open
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
