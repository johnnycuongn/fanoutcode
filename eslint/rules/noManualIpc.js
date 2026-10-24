// teamcode/no-manual-ipc: commands and events go through the generated src/lib/bindings.ts (ADR 0001). Importing
// `invoke`, `listen` or anything else from @tauri-apps/api/core or /event is reported, except `Channel` from core:
// a caller creates one to receive a generated command's stream.

const CORE = "@tauri-apps/api/core";
const EVENT = "@tauri-apps/api/event";

function allowed(node) {
  if (node.source.value !== CORE) return false;
  return (
    node.specifiers.length > 0 &&
    node.specifiers.every(
      (s) => s.type === "ImportSpecifier" && s.imported.type === "Identifier" && s.imported.name === "Channel",
    )
  );
}

/** @type {import("eslint").Rule.RuleModule} */
export default {
  meta: {
    type: "problem",
    docs: { description: "Forbid hand-written IPC outside src/lib/bindings.ts" },
    messages: {
      manual: "Call commands and events through src/lib/bindings.ts (generated); hand-written IPC is forbidden.",
    },
    schema: [],
  },
  create(context) {
    if (context.filename.replaceAll("\\", "/").endsWith("/src/lib/bindings.ts")) return {};
    return {
      ImportDeclaration(node) {
        if ((node.source.value === CORE || node.source.value === EVENT) && !allowed(node)) {
          context.report({ node, messageId: "manual" });
        }
      },
    };
  },
};
