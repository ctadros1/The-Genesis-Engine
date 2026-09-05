// Continue targets the last-viewed world, not simply the first one listed —
// plan: "Continue (the last world viewed)". main.ts persists it per profile
// in localStorage (profiles.ts's getLastWorldId/setLastWorldId) whenever a
// screen treats a world as "the one being looked at" (worlds View, the live
// world switcher, the builder's Create, saves' Branch), and boot() re-reads
// it: the persisted world if the server still lists it, else the first
// listed world, else Continue stays disabled (covered by c1's cold-load
// case, where "Continue" is the first enabled menu entry once world 1
// exists).

import { adminTest as test, expect, stopNonPrimaryWorlds } from "./fixtures";

test.describe("Continue targets the last-viewed world", () => {
  test.afterEach(async () => {
    await stopNonPrimaryWorlds();
  });

  test("viewing world 1, then reloading: Continue leads to world 1", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Worlds" }).click();
    const card = page.locator('[data-world-id="1"]');
    await expect(card).toBeVisible();
    await card.getByRole("button", { name: "View" }).click();
    await expect(page.locator(".live-connection-status")).toContainText("Connected", {
      timeout: 15_000,
    });
    await expect(page.locator(".live-world-name")).toHaveText("primary");

    // A cold reload re-runs main.ts's boot(): the persisted lastWorldId
    // (world 1, written by the View click above) must still be there.
    await page.goto("/");
    await expect(page.locator(".title-menu-panel")).toBeVisible();
    await page.getByRole("button", { name: "Continue" }).click();

    await expect(page.locator(".live-connection-status")).toContainText("Connected", {
      timeout: 15_000,
    });
    await expect(page.locator(".live-world-name")).toHaveText("primary");
  });

  test("creating a second world via the builder, then reloading: Continue leads to it", async ({
    page,
  }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "New World" }).click();
    await expect(page.locator(".builder-groups")).toBeVisible();
    await page.locator(".builder-left input[type=text]").first().fill("Continue Target World");

    const createButton = page.getByRole("button", { name: "Create World" });
    await expect(createButton).toBeEnabled();
    await createButton.click();

    await expect(page.locator(".live-connection-status")).toContainText("Connected", {
      timeout: 15_000,
    });
    await expect(page.locator(".live-world-name")).toHaveText("Continue Target World");

    // Reload: world 1 ("primary") is still the *first* listed world, so a
    // reload that fell back to "first listed" (the old, wrong behaviour)
    // would land Continue on world 1 instead — this is the case that
    // catches that regression.
    await page.goto("/");
    await expect(page.locator(".title-menu-panel")).toBeVisible();
    await page.getByRole("button", { name: "Continue" }).click();

    await expect(page.locator(".live-connection-status")).toContainText("Connected", {
      timeout: 15_000,
    });
    await expect(page.locator(".live-world-name")).toHaveText("Continue Target World");
  });
});
