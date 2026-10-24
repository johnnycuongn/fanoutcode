// ESLint 9 flat config. The `teamcode` rules (eslint/rules/) enforce the repository rules in CLAUDE.md.
import prettier from "eslint-config-prettier";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";
import teamcode from "./eslint/rules/index.js";

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
    ],
  },
  { files: ["**/*.{ts,tsx}"], extends: [tseslint.configs.recommended] },
  { files: ["**/*.{ts,tsx}"], ...reactHooks.configs.flat.recommended },
  {
    files: ["src/**/*.{ts,tsx}"],
    plugins: { teamcode },
    rules: {
      "teamcode/no-raw-controls": "error",
      "teamcode/no-colour-literals": "error",
      "teamcode/no-manual-ipc": "error",
      "teamcode/feature-boundaries": "error",
    },
  },
  prettier,
);
