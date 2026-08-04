import { defineConfig } from "@playwright/test";

// The lifesim-server process is started by scripts/run-observer-e2e.sh with
// fixed test tokens and ports; Playwright manages only the Vite dev server.
export default defineConfig({
  testDir: "./tests",
  timeout: 60_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:5273",
    channel: "chrome",
    headless: true,
    screenshot: "only-on-failure",
  },
  webServer: {
    command: "npm run dev -- --host 127.0.0.1 --port 5273",
    url: "http://127.0.0.1:5273",
    reuseExistingServer: true,
    timeout: 30_000,
  },
});
