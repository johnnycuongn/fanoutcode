import { expect, test } from "vitest";
import type { Stage, TaskEvent } from "@/lib/bindings";
import { memoryFileFor, shouldReload, type MemoryFile } from "./memory";

test.each<[Stage, MemoryFile]>([
  ["Spec", "intent.md"],
  ["Plan", "plan.md"],
  ["Build", "plan.md"],
  ["Review", "review.md"],
  ["Test", "plan.md"],
  ["Commit", "commits.md"],
  ["Done", "index.md"],
])("memoryFileFor(%s)", (s, f) => expect(memoryFileFor(s)).toBe(f));
const post = (tool: string, file_path: string): TaskEvent =>
  ({
    seq: 1,
    at: "",
    task_id: "task-1",
    agent_id: null,
    event: { type: "PostToolUse", tool, input: { file_path }, response: {} },
  }) as TaskEvent;
test("shouldReload only for file tools writing that file, and never while dirty", () => {
  expect(shouldReload(post("Edit", "/m/2026-10-24-a/plan.md"), "plan.md", false)).toBe(true);
  expect(shouldReload(post("Edit", "/m/2026-10-24-a/plan.md"), "plan.md", true)).toBe(false);
  expect(shouldReload(post("Edit", "/m/2026-10-24-a/spec.md"), "plan.md", false)).toBe(false);
  expect(shouldReload(post("Bash", "/m/2026-10-24-a/plan.md"), "plan.md", false)).toBe(false);
});
