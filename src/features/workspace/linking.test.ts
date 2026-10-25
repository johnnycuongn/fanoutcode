import { describe, it, expect } from "vitest";
import { linkingNote, linkStatusMessage, LINKS_CHANGED_MESSAGE, READ_ONLY_LABEL } from "./linking";

describe("linking copy", () => {
  it("notes linking is Claude Code only for codex and shell inside a workspace", () => {
    expect(linkingNote("Codex", true)).toMatch(/Claude Code tasks only/);
    expect(linkingNote("Shell", true)).not.toBeNull();
    expect(linkingNote("ClaudeCode", true)).toBeNull();
    expect(linkingNote("Codex", false)).toBeNull();
  });
  it("maps link status to the banner message", () => {
    expect(linkStatusMessage({ type: "Changed", added: ["/w/x"], removed: [] })).toBe(LINKS_CHANGED_MESSAGE);
    expect(linkStatusMessage({ type: "Bound", workspace_id: null, linked_roots: [] })).toBeNull();
    expect(READ_ONLY_LABEL).toBe("Read only: file tools blocked, shell commands checked");
  });
});
