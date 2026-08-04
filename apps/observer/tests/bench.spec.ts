// Render benchmark (gated): samples browser frame intervals at the dense
// default viewport while connected to a live server. Run through
// scripts/run-phase3-benchmarks.sh with LIFESIM_BENCH=1.

import { test } from "@playwright/test";

const REST_PORT = process.env.LIFESIM_E2E_REST_PORT ?? "8950";
const WS_PORT = process.env.LIFESIM_E2E_WS_PORT ?? "8951";
const OBSERVER_TOKEN = process.env.LIFESIM_E2E_OBSERVER_TOKEN ?? "e2e-observer";

test("render frame interval sampling", async ({ page }) => {
  test.skip(process.env.LIFESIM_BENCH !== "1", "benchmark mode only");
  const params = new URLSearchParams({
    rest: `http://127.0.0.1:${REST_PORT}`,
    ws: `ws://127.0.0.1:${WS_PORT}`,
    token: OBSERVER_TOKEN,
  });
  await page.goto(`/?${params.toString()}`);
  await page.waitForFunction(() => window.__OBS__?.connected === true);
  await page.waitForFunction(() => window.__OBS__.keyframes >= 1);
  // Let the sampler fill its 240-frame window under live streaming.
  await page.waitForTimeout(10_000);
  const result = await page.evaluate(() => {
    const sorted = [...window.__OBS__.frameIntervals].sort((a, b) => a - b);
    const pick = (fraction: number) =>
      sorted[Math.min(sorted.length - 1, Math.ceil((sorted.length - 1) * fraction))];
    return {
      samples: sorted.length,
      population: window.__OBS__.population,
      frame_interval_ms: {
        p50: pick(0.5),
        p95: pick(0.95),
        p99: pick(0.99),
      },
      deltas_received: window.__OBS__.deltas,
    };
  });
  console.log(`RENDER_BENCH ${JSON.stringify(result)}`);
});
