import { describe, it, expect, vi, beforeEach } from "vitest";
vi.mock("@/lib/bindings", () => mockBindings());
import { mockBindings, commands } from "@/test/mockBindings";
import type { ContextDoc } from "@/lib/bindings";
import { contextDocsStore, containerKey } from "./contextDocsStore";

const ok = <T>(data: T) => ({ status: "ok" as const, data });
const doc = (id: string): ContextDoc => ({
  id,
  kind: "Url",
  target: "https://x.y",
  label: "x.y",
  page_id: null,
  fetched_at: null,
  order: 0,
  trusted: true,
});

describe("contextDocsStore", () => {
  beforeEach(() => {
    contextDocsStore.reset();
    vi.clearAllMocks();
  });

  it("keys containers", () => {
    expect(containerKey({ type: "Project", id: "proj-1" })).toBe("project:proj-1");
    expect(containerKey({ type: "Workspace", id: "ws-1" })).toBe("workspace:ws-1");
  });

  it("add appends the returned doc under the container key", async () => {
    commands.contextDocsAdd.mockResolvedValue(ok(doc("doc-1")));
    await contextDocsStore.add({ type: "Project", id: "proj-1" }, "Url", "https://x.y", null);
    expect(contextDocsStore.get().docs["project:proj-1"].map((d) => d.id)).toEqual(["doc-1"]);
  });

  it("fetchText marks fetching while pending", async () => {
    let resolve!: (s: unknown) => void;
    commands.contextDocsFetch.mockReturnValue(new Promise((r) => (resolve = r)));
    commands.contextDocsList.mockResolvedValue(ok([]));
    const p = contextDocsStore.fetchText({ type: "Project", id: "proj-1" }, "doc-1");
    expect(contextDocsStore.get().fetching["doc-1"]).toBe(true);
    resolve(ok("text"));
    await expect(p).resolves.toBe("text");
    expect(contextDocsStore.get().fetching["doc-1"]).toBeFalsy();
  });

  it("trust replaces the doc with the confirmed one", async () => {
    contextDocsStore.set({ docs: { "workspace:ws-1": [{ ...doc("doc-2"), trusted: false }] } });
    commands.contextDocsTrust.mockResolvedValue(ok(doc("doc-2")));
    await contextDocsStore.trust({ type: "Workspace", id: "ws-1" }, "doc-2");
    expect(contextDocsStore.get().docs["workspace:ws-1"][0].trusted).toBe(true);
  });
});
