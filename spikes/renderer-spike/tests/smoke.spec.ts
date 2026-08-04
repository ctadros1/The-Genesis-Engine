import { expect, test } from "@playwright/test";

const cases = [
  { name: "desktop", width: 1280, height: 720 },
  { name: "mobile", width: 390, height: 844 },
] as const;

for (const viewport of cases) {
  test(`${viewport.name} WebGL renderer completes`, async ({ page }) => {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    await page.goto("/?backend=webgl&count=500&warmup=5&frames=15");
    await page.waitForFunction(() => window.__PHASE0_RESULT__ !== undefined || window.__PHASE0_ERROR__ !== undefined);

    const error = await page.evaluate(() => window.__PHASE0_ERROR__);
    expect(error).toBeUndefined();
    const result = await page.evaluate(() => window.__PHASE0_RESULT__);
    expect(result).toBeDefined();
    expect(result?.requested_backend).toBe("webgl");
    expect(result?.actual_backend).toBe("webgl");
    expect(result?.organism_count).toBe(500);
    expect(result?.sampled_frames).toBe(15);
    expect(result?.visible_organisms.p50).toBeGreaterThan(0);
    await expect(page.locator("canvas")).toBeVisible();
  });
}

