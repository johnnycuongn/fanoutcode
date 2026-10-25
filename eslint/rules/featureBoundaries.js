// fanoutcode/feature-boundaries (spec §15.1): a feature's only public surface is its index.ts. Any import that resolves
// into src/features/<b>/ from outside feature <b> must be written `@/features/<b>` or `@/features/<b>/index`.
// Resolution is by path only (`@/` → src/, relative to the importing file); the filesystem is never read.

import path from "node:path";

function srcRoot(filename) {
  const i = filename.lastIndexOf("/src/");
  return i === -1 ? null : filename.slice(0, i + 4);
}

/** The feature a path lies in, or null. */
function featureOf(file, src) {
  const rel = path.posix.relative(`${src}/features`, file);
  if (rel === "" || rel.startsWith("..") || path.posix.isAbsolute(rel)) return null;
  return rel.split("/")[0];
}

/** @type {import("eslint").Rule.RuleModule} */
export default {
  meta: {
    type: "problem",
    docs: { description: "Import another feature only through its index.ts" },
    messages: { boundary: "Import feature {{feature}} through its index.ts only." },
    schema: [],
  },
  create(context) {
    const filename = context.filename.replaceAll("\\", "/");
    const src = srcRoot(filename);
    if (src === null) return {};
    const own = featureOf(filename, src);

    function check(node, source) {
      if (source?.type !== "Literal" || typeof source.value !== "string") return;
      const spec = source.value;
      let target;
      if (spec.startsWith("@/")) target = path.posix.join(src, spec.slice(2));
      else if (spec.startsWith(".")) target = path.posix.join(path.posix.dirname(filename), spec);
      else return;
      const feature = featureOf(target, src);
      if (feature === null || feature === own) return;
      if (spec === `@/features/${feature}` || spec === `@/features/${feature}/index`) return;
      context.report({ node, messageId: "boundary", data: { feature } });
    }

    return {
      ImportDeclaration: (node) => check(node, node.source),
      ExportNamedDeclaration: (node) => check(node, node.source),
      ExportAllDeclaration: (node) => check(node, node.source),
      ImportExpression: (node) => check(node, node.source),
    };
  },
};
