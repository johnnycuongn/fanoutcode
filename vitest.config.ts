import { defineConfig } from "vitest/config";

// Tests run in Node; a test that needs the DOM opts in with `// @vitest-environment jsdom`.
export default defineConfig({
  test: {
    include: ["src/**/*.test.{ts,tsx}", "eslint/**/*.test.{js,ts}"],
    environment: "node",
  },
});
