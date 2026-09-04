// C4: accessibility and reduced motion. The live status region announces
// every screen change; Tab reaches every enabled control on a screen; no
// status is colour-only (every badge carries text); and the title
// background stops animating under prefers-reduced-motion.

import type { Page } from "@playwright/test";
import { adminTest, plainTest, expect } from "./fixtures";

const CANDIDATE_SELECTOR =
  "#screen a[href], #screen button, #screen textarea, #screen input, #screen select, #screen [tabindex]";

/// Tags every control under #screen that a real Tab key would actually
/// land on with a probe attribute, then presses Tab repeatedly and records
/// which probes get focused, asserting every one is reached. "Actually
/// reachable" excludes disabled elements, tabindex="-1", anything with no
/// layout box (CSS/`hidden`-attribute hidden — querySelectorAll alone
/// doesn't know this), and — since the browser's native radio-group
/// behaviour keeps only one radio per `name` group in the tab order — every
/// unchecked radio in a group that has a checked one. Bounded well past one
/// full lap so a wrap-around still finishes inside the loop.
async function assertFocusReachesEveryControl(page: Page): Promise<void> {
  const expectedCount = await page.evaluate((selector) => {
    function isReachable(el: Element): boolean {
      const asAny = el as HTMLElement & { disabled?: boolean; type?: string; checked?: boolean; name?: string };
      if (asAny.disabled) return false;
      if (el.getAttribute("tabindex") === "-1") return false;
      if (el.getClientRects().length === 0) return false;
      if (el.tagName === "INPUT" && asAny.type === "radio" && !asAny.checked) {
        const group = Array.from(
          document.querySelectorAll<HTMLInputElement>(`input[type="radio"][name="${asAny.name}"]`),
        );
        const anyChecked = group.some((radio) => radio.checked);
        if (anyChecked) return false;
        if (group[0] !== el) return false;
      }
      return true;
    }
    const candidates = Array.from(document.querySelectorAll<HTMLElement>(selector)).filter(isReachable);
    candidates.forEach((el, i) => el.setAttribute("data-focus-probe", String(i)));
    (document.activeElement as HTMLElement | null)?.blur();
    return candidates.length;
  }, CANDIDATE_SELECTOR);
  expect(expectedCount).toBeGreaterThan(0);

  const visited = new Set<string>();
  for (let step = 0; step < expectedCount * 2 + 10; step += 1) {
    await page.keyboard.press("Tab");
    const marker = await page.evaluate(
      () => (document.activeElement as HTMLElement | null)?.getAttribute("data-focus-probe") ?? null,
    );
    if (marker !== null) visited.add(marker);
    if (visited.size >= expectedCount) break;
  }
  expect(visited.size).toBe(expectedCount);
}

async function assertNoColourOnlyBadges(page: Page): Promise<void> {
  const texts = await page.evaluate(() =>
    Array.from(document.querySelectorAll<HTMLElement>(".badge, .status-badge")).map((el) =>
      (el.textContent ?? "").trim(),
    ),
  );
  expect(texts.length).toBeGreaterThan(0);
  for (const text of texts) {
    expect(text.length, "a badge rendered with no text — status would be colour-only").toBeGreaterThan(0);
  }
}

adminTest.describe("C4: accessibility", () => {
  adminTest("the live region announces every screen change", async ({ page }) => {
    await page.goto("/");
    const liveStatus = page.locator("#live-status");
    await expect(liveStatus).toHaveText("Genesis Engine");

    await page.getByRole("button", { name: "Worlds" }).click();
    await expect(liveStatus).toHaveText("Worlds");

    await page.getByRole("button", { name: "Back" }).click();
    await expect(liveStatus).toHaveText("Genesis Engine");

    await page.getByRole("button", { name: "Server" }).click();
    await expect(liveStatus).toHaveText("Server");
  });

  adminTest("focus order reaches every control on the title screen", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator(".title-menu-panel")).toBeVisible();
    await assertFocusReachesEveryControl(page);
  });

  adminTest("focus order reaches every control on the server screen", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Server" }).click();
    await expect(page.locator(".server-screen")).toBeVisible();
    await assertFocusReachesEveryControl(page);
  });

  adminTest("focus order reaches every control on the worlds screen", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Worlds" }).click();
    await expect(page.locator(".card").first()).toBeVisible();
    await assertFocusReachesEveryControl(page);
  });

  adminTest("focus order reaches every visible control on the builder screen", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "New World" }).click();
    await expect(page.locator(".builder-groups")).toBeVisible();
    // Fields within a group only mount once expanded (covered by C2);
    // this checks the screen's own top-level controls plus every group
    // toggle reachable before any expansion.
    await assertFocusReachesEveryControl(page);
  });

  adminTest("no status is colour-only: title and worlds badges carry text", async ({ page }) => {
    await page.goto("/");
    await assertNoColourOnlyBadges(page);
    await page.getByRole("button", { name: "Worlds" }).click();
    await expect(page.locator(".card").first()).toBeVisible();
    await assertNoColourOnlyBadges(page);
  });
});

// No seeded profile: the title screen's background only runs its
// procedural drift (rather than a live-world preview whose canvas
// legitimately changes every tick regardless of reduced motion) when there
// is nothing to connect to. emulateMedia is applied before navigation so
// the app's own `matchMedia("(prefers-reduced-motion: reduce)")` check
// already sees it at mount time.
plainTest(
  "the title background canvas is static under prefers-reduced-motion",
  async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/");
    const canvas = page.locator(".title-drift-canvas");
    await expect(canvas).toBeVisible();
    const first = await canvas.evaluate((el: HTMLCanvasElement) => el.toDataURL());
    await page.waitForTimeout(500);
    const second = await canvas.evaluate((el: HTMLCanvasElement) => el.toDataURL());
    expect(second).toBe(first);
  },
);

plainTest(
  "sanity: the title background canvas animates without reduced motion",
  async ({ page }) => {
    await page.goto("/");
    const canvas = page.locator(".title-drift-canvas");
    await expect(canvas).toBeVisible();
    const first = await canvas.evaluate((el: HTMLCanvasElement) => el.toDataURL());
    await page.waitForTimeout(500);
    const second = await canvas.evaluate((el: HTMLCanvasElement) => el.toDataURL());
    expect(second).not.toBe(first);
  },
);
