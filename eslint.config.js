// ESLint 9 flat config. The `fanoutcode` rules (eslint/rules/) enforce the repository rules in CLAUDE.md.
import prettier from "eslint-config-prettier";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";
import fanoutcode from "./eslint/rules/index.js";

export default tseslint.config(
  {
    ignores: [
      "dist/**",
      "target/**",
      "coverage/**",
      "node_modules/**",
      "src-tauri/**",
      "crates/**",
      "docs/**",
      "src/lib/bindings.ts",
      // Nested git worktrees (git-ignored) hold their own copy of the tree.
      ".worktrees/**",
      ".claude/worktrees/**",
    ],
  },
  { files: ["**/*.{ts,tsx}"], extends: [tseslint.configs.recommended] },
  { files: ["**/*.{ts,tsx}"], ...reactHooks.configs.flat.recommended },
  {
    files: ["src/**/*.{ts,tsx}"],
    plugins: { fanoutcode },
    rules: {
      "fanoutcode/no-raw-controls": "error",
      "fanoutcode/no-colour-literals": "error",
      "fanoutcode/no-manual-ipc": "error",
      "fanoutcode/feature-boundaries": "error",
    },
  },
  prettier,
);
