import { render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
vi.mock("@/lib/bindings", async () => ({
  events: (await import("@/test/taskPageFixtures")).fakeEvents,
  commands: { overlapList: vi.fn(async () => ({ status: "ok", data: [] })) },
}));
import { makeTask } from "@/test/taskPageFixtures";
import { createChangesStore } from "../store";
import { fakeIpc } from "../testing/fixtures";
import { ChangesTab } from "./ChangesTab";

test("opens on the first changed file and mounts its note under the line in the editor", async () => {
  const s = createChangesStore(fakeIpc(), { read: () => null, write: () => {} });
  const { container } = render(<ChangesTab task={makeTask()} store={s} homeName="api" />);
  await screen.findByRole("option", { name: /src\/a\.ts/, selected: true });
  await waitFor(() => expect(container.querySelector(".cm-editor")).toBeTruthy());
  await waitFor(() => expect(container.querySelector('[data-note-id="api/src/a.ts:1:0"]')).toBeTruthy());
  expect(screen.getByText("x")).toBeTruthy();
});

test("says it is loading until the change set arrives, then that there are no changes", async () => {
  let release: (v: never[]) => void = () => {};
  const ipc = fakeIpc();
  ipc.gitChanges = vi.fn(() => new Promise<never[]>((r) => (release = r)));
  const s = createChangesStore(ipc, { read: () => null, write: () => {} });
  render(<ChangesTab task={makeTask()} store={s} homeName="api" />);
  expect(await screen.findByText("Loading changes…")).toBeTruthy();
  expect(screen.queryByText("No changes yet")).toBeNull();
  release([]);
  expect(await screen.findByText("No changes yet")).toBeTruthy();
});

test("side by side, switching files shows the new file's base on the left, not the previous file's", async () => {
  const ipc = fakeIpc();
  // the second file's diff arrives a moment after its selection, as it does over IPC
  ipc.gitDiff = vi.fn(async (_t: string, repo: string, path: string) => {
    if (path === "src/b.ts") await new Promise((r) => setTimeout(r, 30));
    return {
      repo_root: repo,
      path,
      status: "Modified" as const,
      before: `base of ${path}`,
      after: `work on ${path}`,
      binary: false,
    };
  });
  const s = createChangesStore(ipc, { read: () => "split", write: () => {} });
  const { container } = render(<ChangesTab task={makeTask()} store={s} homeName="api" />);
  const left = () => container.querySelector(".cm-mergeView .cm-mergeViewEditor")?.textContent ?? "";
  await waitFor(() => expect(left()).toContain("base of src/a.ts"));
  await s.select(makeTask().id, { repo: "/w/web", path: "src/b.ts" });
  await waitFor(() => expect(left()).toContain("base of src/b.ts"));
});
