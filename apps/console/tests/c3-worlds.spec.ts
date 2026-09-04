// C3: the Worlds screen is live and safe. A control action reflects on its
// card within two seconds; every admin action confirms first, naming the
// world; an observer-role profile sees no admin controls at all.
//
// These tests act only on world 1 ("primary", started by the server's own
// flags) so they never touch the max-worlds bound the other specs spend
// against. The Pause/Resume test restores world 1 to running before it
// finishes, and the confirmation test cancels every dialog it opens, so
// world 1's state is unchanged for any spec that runs after this one.

import { adminTest, observerTest, expect } from "./fixtures";

const WORLD_ID = "1";
const WORLD_NAME = "primary";

adminTest.describe("C3: worlds screen — live and safe", () => {
  adminTest(
    "a control action reflects on the card within two seconds",
    async ({ page }) => {
      await page.goto("/");
      await page.getByRole("button", { name: "Worlds" }).click();

      const card = page.locator(`[data-world-id="${WORLD_ID}"]`);
      await expect(card).toBeVisible();
      const badge = card.locator(".status-badge");
      await expect(badge).toHaveText("Running");

      await card.getByRole("button", { name: "Pause", exact: true }).click();
      const pauseDialog = page.getByRole("alertdialog");
      await expect(pauseDialog).toContainText(WORLD_NAME);
      await pauseDialog.getByRole("button", { name: "Pause", exact: true }).click();

      await expect(badge).toHaveText("Paused", { timeout: 2_000 });

      // The server enforces a per-world control rate limit
      // (CONTROL_RATE_LIMIT_MS in crates/sim-server/src/main.rs, 100ms) so
      // a second rapid control call within the window gets 429'd; a real
      // operator's click-confirm-click never lands this fast, but
      // automation does. Clear the window before issuing Resume.
      await page.waitForTimeout(150);

      await card.getByRole("button", { name: "Resume", exact: true }).click();
      const resumeDialog = page.getByRole("alertdialog");
      await expect(resumeDialog).toContainText(WORLD_NAME);
      await resumeDialog.getByRole("button", { name: "Resume", exact: true }).click();

      await expect(badge).toHaveText("Running", { timeout: 2_000 });
    },
  );

  adminTest(
    "every admin action opens a confirmation naming the world",
    async ({ page }) => {
      await page.goto("/");
      await page.getByRole("button", { name: "Worlds" }).click();

      const card = page.locator(`[data-world-id="${WORLD_ID}"]`);
      await expect(card).toBeVisible();

      for (const label of ["Pause", "Save", "Stop"]) {
        await card.getByRole("button", { name: label, exact: true }).click();
        const dialog = page.getByRole("alertdialog");
        await expect(dialog).toBeVisible();
        await expect(dialog).toContainText(WORLD_NAME);
        await dialog.getByRole("button", { name: "Cancel" }).click();
        await expect(dialog).toBeHidden();
      }

      // Cancelling every dialog must leave the world exactly as it was.
      await expect(card.locator(".status-badge")).toHaveText("Running");
    },
  );
});

observerTest.describe("C3: worlds screen — observer role", () => {
  observerTest("an observer-only profile sees no admin controls", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "Worlds" }).click();

    const cards = page.locator(".card");
    // The screen renders "Loading worlds…" until its first refresh()
    // resolves; wait for a real card before counting.
    await expect(cards.first()).toBeVisible();
    const count = await cards.count();
    expect(count).toBeGreaterThan(0);
    for (let i = 0; i < count; i += 1) {
      const actions = cards.nth(i).locator(".card-actions button");
      await expect(actions).toHaveCount(1);
      await expect(actions.first()).toHaveText("View");
    }
  });
});
