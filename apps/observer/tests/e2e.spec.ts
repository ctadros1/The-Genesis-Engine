// Browser E2E: pan, select, controls, reconnect, and mobile smoke against
// a real lifesim-server (started by scripts/run-observer-e2e.sh).

import { expect, test, type Page } from "@playwright/test";

const REST_PORT = process.env.LIFESIM_E2E_REST_PORT ?? "8950";
const WS_PORT = process.env.LIFESIM_E2E_WS_PORT ?? "8951";
const OBSERVER_TOKEN = process.env.LIFESIM_E2E_OBSERVER_TOKEN ?? "e2e-observer";
const ADMIN_TOKEN = process.env.LIFESIM_E2E_ADMIN_TOKEN ?? "e2e-admin";

function appUrl(admin = false): string {
  const params = new URLSearchParams({
    rest: `http://127.0.0.1:${REST_PORT}`,
    ws: `ws://127.0.0.1:${WS_PORT}`,
    token: OBSERVER_TOKEN,
  });
  if (admin) params.set("admin", ADMIN_TOKEN);
  return `/?${params.toString()}`;
}

async function waitConnected(page: Page): Promise<void> {
  await page.waitForFunction(() => window.__OBS__?.connected === true);
  await page.waitForFunction(() => window.__OBS__.keyframes >= 1);
}

test("connects, streams a keyframe and deltas, tick advances", async ({ page }) => {
  await page.goto(appUrl());
  await waitConnected(page);
  const tickBefore = await page.evaluate(() => window.__OBS__.tick.toString());
  await page.waitForFunction(
    (before) => window.__OBS__.tick.toString() !== before,
    tickBefore,
  );
  await page.waitForFunction(() => window.__OBS__.deltas >= 2);
  await expect(page.locator("#viewport canvas")).toBeVisible();
  await expect(page.locator("#status")).toContainText("Connected");
});

test("pan moves the camera; wheel zooms", async ({ page }) => {
  await page.goto(appUrl());
  await waitConnected(page);
  const before = await page.evaluate(() => ({
    x: window.__OBS__.cameraX,
    y: window.__OBS__.cameraY,
    zoom: window.__OBS__.zoom,
  }));
  const canvas = page.locator("#viewport canvas");
  const box = await canvas.boundingBox();
  if (!box) throw new Error("no canvas box");
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 120, box.y + box.height / 2 + 60, { steps: 5 });
  await page.mouse.up();
  const afterPan = await page.evaluate(() => ({
    x: window.__OBS__.cameraX,
    y: window.__OBS__.cameraY,
  }));
  expect(Math.abs(afterPan.x - before.x)).toBeGreaterThan(50);
  await page.mouse.wheel(0, -400);
  const afterZoom = await page.evaluate(() => window.__OBS__.zoom);
  expect(afterZoom).toBeGreaterThan(before.zoom);
});

test("selecting an organism opens the inspector with trait details", async ({ page }) => {
  await page.goto(appUrl());
  await waitConnected(page);
  // Find a live organism's screen position through the test hook.
  const target = await page.evaluate(() => {
    const id = window.__OBS__.anyOrganismId();
    return id ? { id, at: window.__OBS__.screenOf(id) } : null;
  });
  expect(target?.at).toBeTruthy();
  if (!target?.at) return;
  await page.mouse.click(target.at.x, target.at.y);
  await expect(page.locator("#inspector")).toBeVisible();
  await expect(page.locator("#inspect-body")).toContainText("Energy");
  await expect(page.locator("#inspect-body")).toContainText("Sensor range");
  // Scientific overlay toggles without changing simulation state.
  await page.locator("#btn-overlay").click();
  await expect(page.locator("#btn-overlay")).toHaveAttribute("aria-pressed", "true");
});

test("admin pause and resume control the world and are reflected", async ({ page }) => {
  await page.goto(appUrl(true));
  await waitConnected(page);
  await page.locator("#btn-pause").click();
  await expect(page.locator("#status")).toContainText("PAUSED");
  const tickWhenPaused = await page.evaluate(() => window.__OBS__.tick.toString());
  await page.waitForTimeout(700);
  const tickLater = await page.evaluate(() => window.__OBS__.tick.toString());
  expect(tickLater).toBe(tickWhenPaused);
  await page.locator("#btn-resume").click();
  await page.waitForFunction(
    (paused) => window.__OBS__.tick.toString() !== paused,
    tickWhenPaused,
  );
});

test("observer without admin token has controls disabled", async ({ page }) => {
  await page.goto(appUrl(false));
  await waitConnected(page);
  await expect(page.locator("#btn-pause")).toBeDisabled();
  await expect(page.locator("#btn-resume")).toBeDisabled();
});

test("reconnect resyncs with a fresh keyframe", async ({ page }) => {
  await page.goto(appUrl());
  await waitConnected(page);
  const keyframesBefore = await page.evaluate(() => window.__OBS__.keyframes);
  await page.evaluate(() => window.__OBS__.forceCloseSocket());
  await page.waitForFunction(() => window.__OBS__.reconnects >= 1);
  await page.waitForFunction(
    (before) => window.__OBS__.connected && window.__OBS__.keyframes > before,
    keyframesBefore,
  );
  await expect(page.locator("#status")).toContainText("Connected");
});

test("mobile viewport keeps the canvas and controls usable", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(appUrl(true));
  await waitConnected(page);
  await expect(page.locator("#viewport canvas")).toBeVisible();
  await expect(page.locator("#controls")).toBeVisible();
  // Touch-style pan.
  const box = await page.locator("#viewport canvas").boundingBox();
  if (!box) throw new Error("no canvas box");
  const before = await page.evaluate(() => window.__OBS__.cameraX);
  await page.mouse.move(box.x + 200, box.y + 400);
  await page.mouse.down();
  await page.mouse.move(box.x + 120, box.y + 380, { steps: 4 });
  await page.mouse.up();
  const after = await page.evaluate(() => window.__OBS__.cameraX);
  expect(after).not.toBe(before);
});
