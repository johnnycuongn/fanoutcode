import { expect, test, type Mock } from "vitest";
import { createChangesStore, type DiffMode } from "./store";
import { fakeIpc } from "./testing/fixtures";

const sel = { repo: "/w/api", path: "src/a.ts" };

test("open loads changes and notes and selects the first changed file with its diff", async () => {
  const ipc = fakeIpc();
  const s = createChangesStore(ipc);
  await s.open("task-1");
  const st = s.store.get().byTask["task-1"];
  expect(st.repos).toHaveLength(2);
  expect(st.notes).toHaveLength(1);
  expect(st.selected).toEqual(sel);
  expect(ipc.gitDiff).toHaveBeenCalledWith("task-1", "/w/api", "src/a.ts");
  expect(st.buffer).toBe("old");
});
test("save writes the buffer, then re-reads the diff and the change list", async () => {
  const ipc = fakeIpc();
  const s = createChangesStore(ipc);
  await s.open("task-1");
  s.edit("task-1", "new");
  expect(s.store.get().byTask["task-1"].dirty).toBe(true);
  await expect(s.save("task-1")).resolves.toBe(true);
  const order = [ipc.gitFileWrite, ipc.gitDiff, ipc.gitChanges].map((m) =>
    (m as Mock).mock.invocationCallOrder.at(-1)!,
  );
  expect(order[0]).toBeLessThan(order[1]);
  expect(order[1]).toBeLessThan(order[2]);
  expect(ipc.gitFileWrite).toHaveBeenCalledWith("task-1", "/w/api", "src/a.ts", "new");
  expect(s.store.get().byTask["task-1"]).toMatchObject({ dirty: false, staleOnDisk: false });
  expect(s.store.get().byTask["task-1"].diff?.after).toBe("new");
});
test("a git-changes event while dirty marks the file stale instead of reloading; reload discards the buffer", async () => {
  const ipc = fakeIpc();
  const s = createChangesStore(ipc);
  await s.open("task-1");
  s.edit("task-1", "mine");
  ipc.disk = "theirs";
  await s.onGitChanges("task-1");
  expect(s.store.get().byTask["task-1"]).toMatchObject({ buffer: "mine", staleOnDisk: true });
  await s.reload("task-1");
  expect(s.store.get().byTask["task-1"]).toMatchObject({ buffer: "theirs", dirty: false, staleOnDisk: false });
});
test("a git-changes event while clean reloads the list and the diff", async () => {
  const ipc = fakeIpc();
  const s = createChangesStore(ipc);
  await s.open("task-1");
  ipc.disk = "theirs";
  await s.onGitChanges("task-1");
  expect(s.store.get().byTask["task-1"].buffer).toBe("theirs");
  expect(ipc.gitChanges).toHaveBeenCalledTimes(2);
});
test("include pulls an Other change into the set and refreshes", async () => {
  const ipc = fakeIpc();
  const s = createChangesStore(ipc);
  await s.open("task-1");
  await s.include("task-1", { repo: "/w/api", path: "src/stray.ts" });
  expect(ipc.gitIncludeOther).toHaveBeenCalledWith("task-1", "/w/api", "src/stray.ts");
  expect(ipc.gitChanges).toHaveBeenCalledTimes(2);
});
test("reply sends Reply with the prefixed prompt", async () => {
  const ipc = fakeIpc();
  const s = createChangesStore(ipc);
  await s.open("task-1");
  const note = s.store.get().byTask["task-1"].notes[0];
  await s.reply("task-1", note, "Switched to timingSafeEqual.");
  expect(ipc.tasksAction).toHaveBeenCalledWith("task-1", {
    type: "Reply",
    prompt: "Review note api/src/a.ts:1 · high · x\n\nReply from the developer: Switched to timingSafeEqual.",
  });
});
test("a failed save keeps the buffer and reports the error", async () => {
  const ipc = fakeIpc();
  (ipc.gitFileWrite as Mock).mockRejectedValueOnce("Permission denied");
  const s = createChangesStore(ipc);
  await s.open("task-1");
  s.edit("task-1", "new");
  await expect(s.save("task-1")).resolves.toBe(false);
  expect(s.store.get().byTask["task-1"]).toMatchObject({ buffer: "new", dirty: true, error: "Permission denied" });
});
test("the diff mode persists", () => {
  const p = {
    v: null as DiffMode | null,
    read: () => p.v,
    write: (m: DiffMode) => {
      p.v = m;
    },
  };
  const s = createChangesStore(fakeIpc(), p);
  expect(s.store.get().mode).toBe("unified");
  s.setMode("split");
  expect(p.v).toBe("split");
  expect(createChangesStore(fakeIpc(), p).store.get().mode).toBe("split");
});
