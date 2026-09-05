// C3: the Worlds screen is live and safe. A control action reflects on its
// card within two seconds; every admin action confirms first, naming the
// world; an observer-role profile sees no admin controls at all.
//
// These tests act only on world 1 ("primary", started by the server's own
// flags) so they never touch the max-worlds bound the other specs spend
// against. The Pause/Resume test restores world 1 to running before it
// finishes, and the confirmation test cancels every dialog it opens, so
// world 1's state is unchanged for any spec that runs after this one.

import { adminTest, observerTest, expect, stopNonPrimaryWorlds, waitForBuilderReady } from "./fixtures";

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

  observerTest("an observer-only profile sees no Create button in the builder", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "New World" }).click();

    // The observer can still read the schema (explanation banner + every
    // settings group), just never mutate it.
    await expect(page.locator(".readonly-banner")).toBeVisible();
    await expect(page.locator(".builder-groups")).toBeVisible();
    await expect(page.getByRole("button", { name: "Create World" })).toHaveCount(0);
    await expect(page.locator(".builder-footer")).toHaveCount(0);
  });
});

adminTest.describe("C3: worlds screen — delete and saves confirmations", () => {
  // "Delete" creates and then removes its own world, and "Branch" creates
  // one that only this describe block's afterEach ever sees (it starts
  // stopped, so it needs no manual restore); both stay off world 1.
  adminTest.afterEach(async () => {
    await stopNonPrimaryWorlds();
  });

  adminTest(
    "Delete on a stopped world confirms, names the world, and removes its card",
    async ({ page }) => {
      await page.goto("/");
      await page.getByRole("button", { name: "New World" }).click();
      await waitForBuilderReady(page);
      await page.locator(".builder-left input[type=text]").first().fill("Delete Me World");
      const createButton = page.getByRole("button", { name: "Create World" });
      await expect(createButton).toBeEnabled();
      await createButton.click();
      await expect(page.locator(".live-connection-status")).toContainText("Connected", {
        timeout: 15_000,
      });

      // Fresh load so the new world shows up in the Worlds screen's list
      // (a reload also re-derives Continue — covered by continue-last-
      // world.spec.ts, not here).
      await page.goto("/");
      await page.getByRole("button", { name: "Worlds" }).click();
      const card = page.locator(".card", { hasText: "Delete Me World" });
      await expect(card).toBeVisible();

      await card.getByRole("button", { name: "Stop", exact: true }).click();
      const stopDialog = page.getByRole("alertdialog");
      await expect(stopDialog).toContainText("Delete Me World");
      await stopDialog.getByRole("button", { name: "Stop", exact: true }).click();
      await expect(card.locator(".status-badge")).toHaveText("Stopped", { timeout: 2_000 });

      await card.getByRole("button", { name: "Delete", exact: true }).click();
      const deleteDialog = page.getByRole("alertdialog");
      await expect(deleteDialog).toBeVisible();
      await expect(deleteDialog).toContainText("Delete Me World");
      await deleteDialog.getByRole("button", { name: "Delete", exact: true }).click();
      await expect(card).toHaveCount(0);
    },
  );

  adminTest(
    "Save now, Verify and Branch on the Saves screen each confirm and name the world",
    async ({ page }) => {
      await page.goto("/");
      await page.getByRole("button", { name: "Worlds" }).click();
      const card = page.locator(`[data-world-id="${WORLD_ID}"]`);
      await expect(card).toBeVisible();
      // Start listening before the click that triggers savesScreen's mount
      // (and its initial GET) — reading a row-count baseline off the DOM
      // before that fetch lands would see the pre-refresh() empty state.
      const initialSavesLoad = page.waitForResponse(
        (response) => response.url().includes(`/worlds/${WORLD_ID}/saves`) && response.request().method() === "GET",
      );
      await card.getByRole("button", { name: "Branch" }).click();
      await expect(page.locator(".screen-header h1")).toContainText(WORLD_NAME);
      await initialSavesLoad;

      // Save now. The row is found by count rather than by the entered
      // name: the server's save-create route reads "name" from the URL's
      // query string (crates/sim-server/src/main.rs's create_save), but
      // the client currently sends it in the POST body, so every save
      // lands named "manual" regardless — a pre-existing mismatch outside
      // this task's five items. New saves sort first (ORDER BY save_id
      // DESC), so the row this created is always .first().
      const rows = page.locator(".saves-table tbody tr");
      const rowCountBefore = await rows.count();
      await page.getByRole("button", { name: "Save now" }).click();
      const saveDialog = page.getByRole("alertdialog");
      await expect(saveDialog).toBeVisible();
      await expect(saveDialog).toContainText(WORLD_NAME);
      await saveDialog.locator("input[type=text]").fill("c3-coverage-save");
      await saveDialog.getByRole("button", { name: "Save now", exact: true }).click();
      await expect(saveDialog).toBeHidden();
      await expect(rows).toHaveCount(rowCountBefore + 1, { timeout: 10_000 });

      const row = rows.first();

      // Verify
      await row.getByRole("button", { name: "Verify" }).click();
      const verifyDialog = page.getByRole("alertdialog");
      await expect(verifyDialog).toBeVisible();
      await expect(verifyDialog).toContainText(WORLD_NAME);
      await verifyDialog.getByRole("button", { name: "Verify", exact: true }).click();
      await expect(verifyDialog).toBeHidden();
      await expect(page.locator(".verify-report")).toBeVisible();

      // Branch — its dialog is the name-prompt variant, so it carries an
      // input as well as naming the world.
      await row.getByRole("button", { name: "Branch" }).click();
      const branchDialog = page.getByRole("alertdialog");
      await expect(branchDialog).toBeVisible();
      await expect(branchDialog).toContainText(WORLD_NAME);
      await expect(branchDialog.locator("input[type=text]")).toBeVisible();
      await branchDialog.locator("input[type=text]").fill("c3-coverage-branch");
      await branchDialog.getByRole("button", { name: "Branch", exact: true }).click();

      await expect(page.locator(".live-connection-status")).toContainText("Connected", {
        timeout: 15_000,
      });
    },
  );
});
