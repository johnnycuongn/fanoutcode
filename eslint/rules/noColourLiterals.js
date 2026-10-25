// fanoutcode/no-colour-literals: colours live in src/styles.css as tokens (spec §15.3). Reports hex colours in strings
// and templates, and inline colour styles (colour properties or CSS custom properties). No file is exempt.

const HEX = /#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})\b/i;
const COLOUR_KEYS = new Set([
  "color",
  "backgroundColor",
  "background",
  "borderColor",
  "outlineColor",
  "fill",
  "stroke",
  "caretColor",
  "accentColor",
]);

function keyName(property) {
  if (property.type !== "Property" || property.computed) return null;
  if (property.key.type === "Identifier") return property.key.name;
  if (property.key.type === "Literal" && typeof property.key.value === "string") return property.key.value;
  return null;
}

/** @type {import("eslint").Rule.RuleModule} */
export default {
  meta: {
    type: "problem",
    docs: { description: "Forbid hex colours and inline colour styles outside src/styles.css" },
    messages: { colour: "Colours live in src/styles.css as tokens; use a semantic class." },
    schema: [],
  },
  create(context) {
    return {
      Literal(node) {
        if (typeof node.value === "string" && HEX.test(node.value)) context.report({ node, messageId: "colour" });
      },
      TemplateElement(node) {
        if (HEX.test(node.value.raw)) context.report({ node, messageId: "colour" });
      },
      JSXAttribute(node) {
        if (node.name.type !== "JSXIdentifier" || node.name.name !== "style") return;
        const expression = node.value?.type === "JSXExpressionContainer" ? node.value.expression : null;
        if (expression?.type !== "ObjectExpression") return;
        for (const property of expression.properties) {
          const key = keyName(property);
          if (key !== null && (COLOUR_KEYS.has(key) || key.startsWith("--"))) {
            context.report({ node: property, messageId: "colour" });
          }
        }
      },
    };
  },
};
