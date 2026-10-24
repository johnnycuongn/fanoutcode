import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

// Webview tests run in jsdom (src/test/setup.ts fills the gaps Radix touches); a test that needs Node alone (ESLint,
// the filesystem) opts out with `// @vitest-environment node`.
export default defineConfig({
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  test: {
    include: ["src/**/*.test.{ts,tsx}", "eslint/**/*.test.{js,ts}"],
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    css: false,
  },
});
