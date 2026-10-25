// The smoke suite (spec §15.4, A9): the board, the task page and the Team canvas in WebKit (the engine the macOS app
// runs in), against the Vite dev server in `e2e` mode, where src/main.tsx replaces the Tauri IPC with in-page mocks.
// Browsers live in node_modules (`PLAYWRIGHT_BROWSERS_PATH=0`, set by `npm run e2e`), never in a global cache.
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "e2e",
  fullyParallel: true,
  retries: process.env.CI ? 1 : 0,
  use: { baseURL: "http://localhost:1420", trace: "retain-on-failure" },
  projects: [{ name: "webkit", use: devices["Desktop Safari"] }],
  webServer: {
    command: "npm run dev -- --mode e2e --port 1420 --strictPort",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
