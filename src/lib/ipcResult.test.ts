import { expect, test, vi } from "vitest";
vi.mock("@/lib/bindings", async () => (await import("@/test/mockBindings")).mockBindings());
import { commands } from "@/test/mockBindings";
import { live, unwrap } from "./ipcResult";

test("unwrap returns the data, throws the error string and passes plain values through", () => {
  expect(unwrap({ status: "ok", data: 3 })).toBe(3);
  expect(() => unwrap({ status: "error", error: "nope" })).toThrow("nope");
  const diff = { status: "Modified", path: "a" };
  expect(unwrap(diff)).toBe(diff);
});

test("live calls the generated command at call time and unwraps its result", async () => {
  const ipc = live("memoryRead");
  commands.memoryRead.mockResolvedValueOnce({ status: "ok", data: "# Spec" });
  await expect(ipc.memoryRead("task-1", "spec.md")).resolves.toBe("# Spec");
  commands.memoryRead.mockResolvedValueOnce({ status: "error", error: "Task memory folder is missing: /m" });
  await expect(ipc.memoryRead("task-1", "spec.md")).rejects.toBe("Task memory folder is missing: /m");
});
