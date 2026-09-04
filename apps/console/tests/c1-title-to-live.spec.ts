// C1: from a cold load with a saved (active) profile, Title -> New World ->
// Create -> the live view, once with keyboard alone and once with pointer
// alone, against the real lifesim-server started by
// scripts/run-console-e2e.sh.

import { adminTest as test, expect, stopNonPrimaryWorlds } from "./fixtures";

test.describe("C1: title to live view", () => {
  test.afterEach(async () => {
    // Keep world count and tick-thread contention low for later specs
    // (notably C3's two-second reflection budget) — stop, not delete, so
    // the world this test created stays inspectable.
    await stopNonPrimaryWorlds();
  });

  test("keyboard only: New World -> type a name -> Create -> live view", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator(".title-menu-panel")).toBeVisible();

    // focusFirst() lands on the first *enabled* menu entry. A world already
    // exists (world 1, started by the server's own flags) so "Continue" is
    // enabled and is that entry; the menu order is Continue, Worlds, New
    // World, ... so two ArrowDowns reach "New World" without ever touching
    // the mouse.
    await expect(page.getByRole("button", { name: "Continue" })).toBeFocused();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(page.getByRole("button", { name: "New World" })).toBeFocused();
    await page.keyboard.press("Enter");

    await expect(page.locator(".builder-groups")).toBeVisible();
    await expect(page.getByRole("button", { name: "Back" })).toBeFocused();

    // The Name field is the very next focusable control after Back in the
    // builder's DOM order (builder.ts's leftColumn: Name, Preset, Recipe,
    // Seed — Back sits in the header just above it).
    await page.keyboard.press("Tab");
    await expect(page.locator(".builder-left input[type=text]").first()).toBeFocused();
    await page.keyboard.type("Keyboard World");

    const createButton = page.getByRole("button", { name: "Create World" });
    await expect(createButton).toBeEnabled();

    // Tab forward until focus reaches Create. A disabled button is skipped
    // by native Tab handling, so this only lands once Create is enabled
    // (already awaited above); bounded well above one full lap of the
    // form's controls.
    for (let step = 0; step < 80; step += 1) {
      const onCreate = await page.evaluate(
        () => document.activeElement?.textContent?.trim() === "Create World",
      );
      if (onCreate) break;
      await page.keyboard.press("Tab");
    }
    await expect(createButton).toBeFocused();
    await page.keyboard.press("Enter");

    await expect(page.locator(".live-connection-status")).toContainText("Connected", {
      timeout: 15_000,
    });
    await expect(page.locator(".live-viewport canvas")).toBeVisible();
  });

  test("pointer only: New World -> fill a name -> Create -> live view", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "New World" }).click();

    await expect(page.locator(".builder-groups")).toBeVisible();
    await page.locator(".builder-left input[type=text]").first().fill("Pointer World");

    const createButton = page.getByRole("button", { name: "Create World" });
    await expect(createButton).toBeEnabled();
    await createButton.click();

    await expect(page.locator(".live-connection-status")).toContainText("Connected", {
      timeout: 15_000,
    });
    await expect(page.locator(".live-viewport canvas")).toBeVisible();
  });
});
