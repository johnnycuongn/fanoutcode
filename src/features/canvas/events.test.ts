import { describe, it, expect } from "vitest";
import { kindOf, agentOf, pathOf, streamKind, toolTarget, pairToolCalls, normalizeEvents } from "./events";
import { fx, sampleBuild } from "./fixtures";

describe("events", () => {
  it("classifies every hook kind", () => {
    expect(kindOf(fx.sessionStart(1))).toBe("session-start");
    expect(kindOf(fx.pre(2, "a1", "Read", { file_path: "/x" }))).toBe("pre-tool");
    expect(kindOf(fx.agentStart(3, "a1", "frontend"))).toBe("agent-start");
    expect(kindOf(fx.unknown(4, "Fancy"))).toBe("unknown");
  });
  it("attributes events without an agent id to main", () => {
    expect(agentOf(fx.prompt(1))).toBe("main");
    expect(agentOf(fx.edit(2, "a1", "/repo/app/x.ts", "", "x"))).toBe("a1");
  });
  it("extracts the path for edit tools, including notebooks", () => {
    expect(pathOf("Edit", { file_path: "/a" })).toBe("/a");
    expect(pathOf("NotebookEdit", { notebook_path: "/n.ipynb" })).toBe("/n.ipynb");
    expect(pathOf("Bash", { command: "ls" })).toBeNull();
  });
  it("maps tools to stream kinds", () => {
    expect(streamKind("Write")).toBe("edit");
    expect(streamKind("Bash")).toBe("command");
    expect(streamKind("Task")).toBe("agent");
    expect(streamKind("Grep")).toBe("read");
  });
  it("formats a tool target and cuts long commands to 60 chars", () => {
    expect(toolTarget("Edit", { file_path: "/repo/app/src/a.ts" })).toBe("/repo/app/src/a.ts");
    expect(toolTarget("Bash", { command: "x".repeat(80) })).toHaveLength(60);
    expect(toolTarget("Task", { description: "Step 1 · frontend: go" })).toBe("Step 1 · frontend: go");
  });
  it("pairs a PreToolUse with the next PostToolUse of the same agent and tool", () => {
    const calls = pairToolCalls(sampleBuild());
    const bash = calls.find((c) => c.tool === "Bash")!;
    expect(bash).toMatchObject({
      preSeq: 12,
      postSeq: 13,
      agentId: "a2",
      durationMs: 1000,
      kind: "command",
      target: "npm test",
    });
  });
  it("turns an unmatched PostToolUse into a call with no duration", () => {
    const [call] = pairToolCalls([fx.edit(5, "a1", "/repo/app/src/a.ts", "", "x")]);
    expect(call).toMatchObject({ preSeq: 5, postSeq: 5, durationMs: null, kind: "edit" });
  });
  it("normalizeEvents drops duplicate seqs and sorts ascending", () => {
    const out = normalizeEvents([fx.prompt(3), fx.sessionStart(1), fx.prompt(3)]);
    expect(out.map((e) => e.seq)).toEqual([1, 3]);
  });
  it("fixtures spell HookEvent with its type tag", () => {
    expect(fx.prompt(1).event).toEqual({ type: "UserPromptSubmit" });
    expect(fx.agentStart(2, "a1", "frontend").event).toEqual({
      type: "SubagentStart",
      agent_id: "a1",
      agent_type: "frontend",
      description: null,
    });
    expect(fx.edit(3, "a1", "/x", "", "y").event).toEqual({
      type: "PostToolUse",
      tool: "Edit",
      input: { file_path: "/x", old_string: "", new_string: "y" },
      response: {},
    });
  });
});
