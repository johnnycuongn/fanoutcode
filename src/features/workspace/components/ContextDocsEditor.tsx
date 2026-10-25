// The references of a project or workspace: files, folders, links and Confluence pages, with Fetch for links and
// Confirm for those that came from a workspace file. Plan 06's Docs sidebar and project settings reuse it.
import { FileText, Folder, Globe, BookOpen } from "lucide-react";
import { useEffect, useState, type ReactElement } from "react";
import { Button, Input, Segmented, toast } from "@/components/ui";
import type { Container, ContextDoc, ContextDocKind } from "@/lib/bindings";
import { containerFromKey, containerKey, contextDocsStore } from "../contextDocsStore";
import { pickFile, pickFolder } from "../pickers";
import { messageOf } from "../result";

const KINDS: Array<{ value: ContextDocKind; label: string }> = [
  { value: "File", label: "File" },
  { value: "Folder", label: "Folder" },
  { value: "Url", label: "Link" },
  { value: "Confluence", label: "Confluence" },
];
const ICON = { File: FileText, Folder, Url: Globe, Confluence: BookOpen } as const;

function Row({ container, doc }: { container: Container; doc: ContextDoc }): ReactElement {
  const fetching = contextDocsStore.useSelector((s) => s.fetching[doc.id] ?? false);
  const Icon = ICON[doc.kind];
  const run = (p: Promise<unknown>) => void p.catch((e) => toast.error(messageOf(e)));
  return (
    <li className="flex items-center gap-2 text-sm">
      <Icon aria-hidden className="size-3.5 shrink-0 text-mute" />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-ink">{doc.label}</span>
        <span className="block truncate font-mono text-xs text-mute">{doc.target}</span>
      </span>
      <span className="shrink-0 text-xs text-mute">
        {!doc.trusted
          ? "Waiting for confirmation"
          : doc.fetched_at
            ? new Date(doc.fetched_at).toLocaleString()
            : "Not fetched"}
      </span>
      {!doc.trusted && (
        <Button size="sm" variant="secondary" onClick={() => run(contextDocsStore.trust(container, doc.id))}>
          Confirm
        </Button>
      )}
      {doc.trusted && (doc.kind === "Url" || doc.kind === "Confluence") && (
        <Button
          size="sm"
          variant="ghost"
          loading={fetching}
          onClick={() => run(contextDocsStore.fetchText(container, doc.id))}
        >
          Fetch
        </Button>
      )}
      <Button size="sm" variant="ghost" onClick={() => run(contextDocsStore.remove(container, doc.id))}>
        Remove
      </Button>
    </li>
  );
}

export function ContextDocsEditor({ container }: { container: Container }): ReactElement {
  const key = containerKey(container);
  const docs = contextDocsStore.useSelector((s) => s.docs[key]) ?? [];
  const [kind, setKind] = useState<ContextDocKind>("Url");
  const [target, setTarget] = useState("");
  const [label, setLabel] = useState("");
  useEffect(() => {
    void contextDocsStore.load(containerFromKey(key)).catch(() => undefined);
  }, [key]);
  const add = async (t = target) => {
    if (!t.trim()) return;
    try {
      await contextDocsStore.add(container, kind, t.trim(), label.trim() || null);
      setTarget("");
      setLabel("");
    } catch (e) {
      toast.error(messageOf(e));
    }
  };
  const browse = async () => {
    const p = kind === "Folder" ? await pickFolder("Add a reference folder") : await pickFile("Add a reference file");
    if (p) await add(p);
  };
  const isPath = kind === "File" || kind === "Folder";
  return (
    <div className="flex flex-col gap-2">
      <ul className="flex flex-col gap-1" aria-label="References">
        {docs.map((d) => (
          <Row key={d.id} container={container} doc={d} />
        ))}
      </ul>
      <Segmented<ContextDocKind>
        size="sm"
        aria-label="Reference kind"
        options={KINDS}
        value={kind}
        onValueChange={setKind}
      />
      <div className="flex gap-1">
        {isPath ? (
          <Button size="sm" variant="secondary" onClick={() => void browse()}>
            {kind === "Folder" ? "Choose folder…" : "Choose file…"}
          </Button>
        ) : (
          <>
            <Input
              size="sm"
              aria-label="Link"
              placeholder="https://…"
              value={target}
              onChange={(e) => setTarget(e.target.value)}
            />
            <Input
              size="sm"
              aria-label="Label"
              placeholder="Label (optional)"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
            />
            <Button size="sm" variant="secondary" onClick={() => void add()}>
              Add
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
