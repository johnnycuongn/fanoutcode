// The references of each project and workspace (`context_docs_*`), keyed by container; `context-docs-changed`
// reloads the container it names.
import { commands, events, type Container, type ContextDoc, type ContextDocKind } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";
import { unwrap } from "./result";

export type ContextDocsState = { docs: Record<string, ContextDoc[]>; fetching: Record<string, boolean> };

export function containerKey(c: Container): string {
  return c.type === "Project" ? `project:${c.id}` : `workspace:${c.id}`;
}

/** The inverse of `containerKey`. */
export function containerFromKey(key: string): Container {
  const [kind, ...rest] = key.split(":");
  const id = rest.join(":");
  return kind === "project" ? { type: "Project", id } : { type: "Workspace", id };
}

const base = createStore<ContextDocsState>({ docs: {}, fetching: {} });
let listening = false;

function listen() {
  if (listening) return;
  listening = true;
  void events.contextDocsChanged.listen((e) => {
    if (containerKey(e.payload.container) in base.get().docs) void contextDocsStore.load(e.payload.container);
  });
}

const put = (c: Container, docs: ContextDoc[]) =>
  base.set((s) => ({ ...s, docs: { ...s.docs, [containerKey(c)]: docs } }));
const docsOf = (c: Container) => base.get().docs[containerKey(c)] ?? [];
const setFetching = (id: string, on: boolean) => base.set((s) => ({ ...s, fetching: { ...s.fetching, [id]: on } }));

export const contextDocsStore = {
  ...base,
  async load(c: Container): Promise<void> {
    listen();
    put(c, unwrap(await commands.contextDocsList(c)));
  },
  async add(c: Container, kind: ContextDocKind, target: string, label: string | null): Promise<ContextDoc> {
    listen();
    const doc = unwrap(await commands.contextDocsAdd(c, kind, target, label));
    put(c, [...docsOf(c).filter((d) => d.id !== doc.id), doc]);
    return doc;
  },
  async remove(c: Container, id: string): Promise<void> {
    unwrap(await commands.contextDocsRemove(c, id));
    put(
      c,
      docsOf(c).filter((d) => d.id !== id),
    );
  },
  /** Confirms a reference that came from a workspace file. */
  async trust(c: Container, id: string): Promise<void> {
    const doc = unwrap(await commands.contextDocsTrust(c, id));
    put(
      c,
      docsOf(c).map((d) => (d.id === id ? doc : d)),
    );
  },
  /** Fetches the text now (network or disk), for a preview; the list refreshes with the new fetched time. */
  async fetchText(c: Container, id: string): Promise<string> {
    setFetching(id, true);
    try {
      return unwrap(await commands.contextDocsFetch(c, id));
    } finally {
      setFetching(id, false);
      void contextDocsStore.load(c).catch(() => undefined);
    }
  },
};
