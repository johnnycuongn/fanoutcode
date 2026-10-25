// fanoutcode/no-raw-controls: raw form and control elements appear only under src/components/ui/ (spec §15.3).
// Plan 02 may extend PRIMITIVES (the tag → primitive mapping), not the mechanism.

export const PRIMITIVES = {
  input: "Input",
  select: "Select",
  textarea: "Textarea",
  button: "Button",
  dialog: "Dialog",
};

/** @type {import("eslint").Rule.RuleModule} */
export default {
  meta: {
    type: "problem",
    docs: { description: "Forbid raw <input>, <select>, <textarea>, <button> and <dialog> outside src/components/ui/" },
    messages: { raw: "Use the <{{name}}> primitive from src/components/ui instead of a raw <{{tag}}>." },
    schema: [],
  },
  create(context) {
    if (context.filename.replaceAll("\\", "/").includes("/src/components/ui/")) return {};
    return {
      JSXOpeningElement(node) {
        if (node.name.type !== "JSXIdentifier") return;
        const tag = node.name.name;
        if (Object.hasOwn(PRIMITIVES, tag)) {
          context.report({ node, messageId: "raw", data: { name: PRIMITIVES[tag], tag } });
        }
      },
    };
  },
};
