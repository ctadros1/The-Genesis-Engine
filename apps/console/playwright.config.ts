import { defineConfig } from "@playwright/test";

// The lifesim-server process is started by scripts/run-console-e2e.sh with
// fixed test tokens and ports; Playwright manages only the Vite dev server.
// Mirrors apps/observer/playwright.config.ts.
export default defineConfig({
  testDir: "./tests",
  timeout: 60_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:5281",
    channel: "chrome",
    headless: true,
    screenshot: "only-on-failure",
  },
  webServer: {
    command: "npm run dev -- --host 127.0.0.1 --port 5281",
    url: "http://127.0.0.1:5281",
    reuseExistingServer: true,
    timeout: 30_000,
  },
});
