// What the sidebar's Docs section shows for a window's container: its trusted references as rows, and adding a
// dropped file or folder (the shell decides which by what is on disk).
import { useEffect, useMemo } from "react";
import { toast } from "@/components/ui";
import type { Container } from "@/lib/bindings";
import { containerFromKey, containerKey, contextDocsStore } from "./contextDocsStore";
import { messageOf } from "./result";

export type DocRow = { id: string; kind: "file" | "folder" | "url" | "confluence"; label: string; href: string };

const KIND = { File: "file", Folder: "folder", Url: "url", Confluence: "confluence" } as const;
const EMPTY: DocRow[] = [];

/** The container's trusted references as Docs rows. */
export function useDocRows(container: Container | null): DocRow[] {
  const key = container ? containerKey(container) : "";
  const docs = contextDocsStore.useSelector((s) => (key ? s.docs[key] : undefined));
  useEffect(() => {
    if (key) void contextDocsStore.load(containerFromKey(key)).catch(() => undefined);
  }, [key]);
  return useMemo(
    () =>
      docs
        ? docs.filter((d) => d.trusted).map((d) => ({ id: d.id, kind: KIND[d.kind], label: d.label, href: d.target }))
        : EMPTY,
    [docs],
  );
}

/** Adds a dropped path as a reference of the container. */
export function addDroppedPath(container: Container, path: string): void {
  contextDocsStore.add(container, "File", path, null).catch((e) => toast.error(messageOf(e)));
}
